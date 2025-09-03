use std::{pin::pin, sync::Arc};

use alloy_primitives::B256;
use anyhow::Result;
use futures::StreamExt;
use sp1_sdk::{
    NetworkSigner,
    network::{
        NetworkClient,
        proto::{
            artifact::ArtifactType,
            network::prover_network_client::ProverNetworkClient,
            types::{
                CreateProgramRequest, CreateProgramResponse, FulfillmentStatus, GetNonceRequest,
                GetNonceResponse, GetProgramRequest, GetProgramResponse,
                GetProofRequestStatusRequest, GetProofRequestStatusResponse, ProofMode,
                RequestProofRequest, RequestProofResponse, RequestProofResponseBody,
            },
        },
    },
};
use tonic::{Request, Response, Status, transport::Channel};

use crate::{
    db::{ArtifactId, Db},
    fulfiller::Fulfiller,
    types::{Key, PendingRequest, Request as ProofRequest, prover_network_server::ProverNetwork},
    utils::{PresignedUrl, configure_endpoint},
};

#[derive(Debug, Clone)]
pub struct DefaultPrivateProverServer<DB: Db> {
    hostname: String,
    network_rpc_url: String,
    db: Arc<DB>,
}

impl<DB: Db> DefaultPrivateProverServer<DB> {
    pub fn new(
        hostname: String,
        network_rpc_url: String,
        network_private_key: String,
        programs_s3_region: String,
        db: Arc<DB>,
        worker_count: usize,
    ) -> Self {
        spawn_workers(
            db.clone(),
            network_rpc_url.clone(),
            network_private_key,
            programs_s3_region,
            worker_count,
        );

        Self {
            hostname,
            network_rpc_url,
            db,
        }
    }

    async fn prover_network_client(&self) -> Result<ProverNetworkClient<Channel>> {
        let channel = configure_endpoint(&self.network_rpc_url)?.connect().await?;
        Ok(ProverNetworkClient::new(channel))
    }
}

#[tonic::async_trait]
impl<DB: Db> ProverNetwork for DefaultPrivateProverServer<DB> {
    async fn create_program(
        &self,
        request: Request<CreateProgramRequest>,
    ) -> Result<Response<CreateProgramResponse>, Status> {
        let request = request.into_inner();
        let mut network_client = self.prover_network_client().await.unwrap();
        let response_from_network = network_client.create_program(request).await?;

        Ok(response_from_network)
    }

    async fn get_program(
        &self,
        request: Request<GetProgramRequest>,
    ) -> Result<Response<GetProgramResponse>, Status> {
        let request = request.into_inner();
        let mut network_client = self.prover_network_client().await.unwrap();

        network_client.get_program(request).await
    }

    async fn get_nonce(
        &self,
        request: Request<GetNonceRequest>,
    ) -> Result<Response<GetNonceResponse>, Status> {
        let mut network_client = self.prover_network_client().await.unwrap();

        network_client.get_nonce(request).await
    }

    async fn request_proof(
        &self,
        request: Request<RequestProofRequest>,
    ) -> Result<Response<RequestProofResponse>, Status> {
        tracing::debug!("Start request proof");
        let request = request.into_inner();
        let request_body = request
            .body
            .clone()
            .ok_or_else(|| Status::invalid_argument("missing request body"))?;

        let mut network_client = self.prover_network_client().await.unwrap();

        tracing::debug!("Forward proof request to the network");
        let response_from_network = network_client.request_proof(request).await?.into_inner();
        let response_body = response_from_network
            .body
            .clone()
            .ok_or_else(|| Status::invalid_argument("missing networs response body"))?;

        let request_id = B256::from_slice(&response_body.request_id);
        let mode = ProofMode::try_from(request_body.mode)
            .map_err(|_| Status::invalid_argument("missing proof mode"))?;

        self.db
            .update_artifact_id(
                Key::from_uri(&request_body.stdin_uri),
                ArtifactId::RequestId(request_id),
            )
            .await;

        let inputs = self
            .db
            .get_inputs(request_id)
            .await
            .ok_or_else(|| Status::invalid_argument("missing stdin"))?;

        let request = PendingRequest::from_request_body(&request_body, request_id, mode, inputs);
        let response = RequestProofResponse {
            tx_hash: B256::random().to_vec(), // TODO: Impl
            body: Some(RequestProofResponseBody {
                request_id: request.id.to_vec(),
            }),
        };
        self.db.insert_request(request).await;

        Ok(Response::new(response))
    }

    async fn get_proof_request_status(
        &self,
        request: Request<GetProofRequestStatusRequest>,
    ) -> Result<Response<GetProofRequestStatusResponse>, Status> {
        let request_id = request.into_inner().request_id;
        let request = self
            .db
            .get_request(&request_id)
            .await
            .ok_or_else(|| Status::not_found("The request hasn't been requested"))?;

        let fulfillment_status = match request.as_ref() {
            ProofRequest::Assigned => FulfillmentStatus::Assigned,
            ProofRequest::Fulfilled { .. } => FulfillmentStatus::Fulfilled,
            ProofRequest::Unfulfillable { .. } => FulfillmentStatus::Unfulfillable,
        };

        let proof_uri = match request.as_ref() {
            ProofRequest::Fulfilled { proof } => {
                let presigned = PresignedUrl::new(&ArtifactType::Proof);
                self.db
                    .insert_artifact(presigned.key.clone(), proof.as_ref().into())
                    .await;
                Some(presigned.url(&self.hostname))
            }
            _ => None,
        };

        let response = GetProofRequestStatusResponse {
            fulfillment_status: fulfillment_status.into(),
            execution_status: 0,
            request_tx_hash: vec![],
            deadline: u64::MAX,
            fulfill_tx_hash: None,
            proof_uri,
            public_values_hash: None,
            proof_public_uri: None,
        };

        Ok(Response::new(response))
    }
}

fn spawn_workers<DB: Db>(
    db: Arc<DB>,
    network_rpc_url: String,
    network_private_key: String,
    programs_s3_region: String,
    worker_count: usize,
) {
    tokio::spawn(async move {
        let mut pending_requests = pin!(db.get_requests_to_process_stream());
        let network_client = NetworkClient::new(
            NetworkSigner::local(&network_private_key).unwrap(),
            network_rpc_url,
        );
        let network_client = Arc::new(network_client);
        let (tx, rx) = crossbeam::channel::unbounded::<PendingRequest>();

        for gpu_id in 0..worker_count {
            let db = db.clone();
            let rx = rx.clone();
            let network_client = network_client.clone();
            let programs_s3_region = programs_s3_region.clone();

            tokio::spawn(async move {
                while let Ok(request) = rx.recv() {
                    db.set_request_as_assigned(request.id).await;

                    tracing::debug!("Get program {}", request.vk_hash);
                    let pk = db.get_proving_key(request.vk_hash).await;

                    #[cfg(not(feature = "mock"))]
                    let fulfiller = Fulfiller::new(
                        pk,
                        request.clone(),
                        gpu_id,
                        db.clone(),
                        network_client.clone(),
                        programs_s3_region.clone(),
                    );

                    #[cfg(feature = "mock")]
                    let fulfiller = Fulfiller::mock(
                        pk,
                        request.clone(),
                        db.clone(),
                        network_client.clone(),
                        programs_s3_region.clone(),
                    );

                    match fulfiller.process().await {
                        Ok(proof) => {
                            tracing::info!("Proved {}", request.id);
                            db.set_request_as_fulfilled(request.id, proof).await;
                        }
                        Err(reason) => {
                            tracing::error!("Failed to prove {}: {reason}", request.id);
                            db.set_request_as_unfulfillable(request.id, reason).await;
                        }
                    }
                }
            });
        }

        while let Some(request) = pending_requests.next().await {
            tx.send(request).unwrap();
        }
    });
}
