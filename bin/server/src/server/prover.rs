use std::{pin::pin, sync::Arc};

use alloy_primitives::B256;
use anyhow::Result;
use futures::StreamExt;
use sp1_sdk::{
    NetworkSigner,
    network::proto::types::{
        CreateProgramRequest, CreateProgramResponse, FulfillmentStatus, GetNonceRequest,
        GetNonceResponse, GetProgramRequest, GetProgramResponse, GetProofRequestStatusRequest,
        GetProofRequestStatusResponse, ProofMode, RequestProofRequest, RequestProofResponse,
        RequestProofResponseBody,
    },
};
use tonic::{Request, Response, Status};

use crate::{
    db::Db,
    fulfiller::Fulfiller,
    types::{Key, PendingRequest, prover_network_server::ProverNetwork},
    utils::prover_network_client,
};

#[derive(Debug, Clone)]
pub struct DefaultPrivateProverServer<DB: Db> {
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
            hostname,
            worker_count,
        );

        Self {
            network_rpc_url,
            db,
        }
    }
}

#[tonic::async_trait]
impl<DB: Db> ProverNetwork for DefaultPrivateProverServer<DB> {
    async fn create_program(
        &self,
        request: Request<CreateProgramRequest>,
    ) -> Result<Response<CreateProgramResponse>, Status> {
        let request = request.into_inner();
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();
        let response_from_network = network_client.create_program(request).await?;

        Ok(response_from_network)
    }

    async fn get_program(
        &self,
        request: Request<GetProgramRequest>,
    ) -> Result<Response<GetProgramResponse>, Status> {
        let request = request.into_inner();
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        network_client.get_program(request).await
    }

    async fn get_nonce(
        &self,
        request: Request<GetNonceRequest>,
    ) -> Result<Response<GetNonceResponse>, Status> {
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

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

        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        tracing::debug!("Forward proof request to the network");
        let response_from_network = network_client.request_proof(request).await?.into_inner();
        let response_body = response_from_network
            .body
            .clone()
            .ok_or_else(|| Status::invalid_argument("missing networs response body"))?;

        let request_id = B256::from_slice(&response_body.request_id);
        let mode = ProofMode::try_from(request_body.mode)
            .map_err(|_| Status::invalid_argument("missing proof mode"))?;

        let stdin = self
            .db
            .get_stdin(Key::from_uri(&request_body.stdin_uri))
            .await
            .ok_or_else(|| Status::invalid_argument("missing stdin"))?;

        let request = PendingRequest::from_request_body(&request_body, request_id, mode, stdin);
        let response = RequestProofResponse {
            tx_hash: response_from_network.tx_hash.clone(),
            body: Some(RequestProofResponseBody {
                request_id: request.id.to_vec(),
            }),
        };

        self.db.insert_pending_request(request).await;
        self.db
            .insert_request(
                request_id,
                response_from_network.tx_hash,
                request_body.deadline,
            )
            .await;

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

        let response = GetProofRequestStatusResponse {
            fulfillment_status: request.fulfillment_status as i32,
            execution_status: request.execution_status as i32,
            request_tx_hash: request.request_tx_hash,
            deadline: request.deadline,
            fulfill_tx_hash: request.fulfill_tx_hash,
            proof_uri: request.proof_uri,
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
    hostname: String,
    worker_count: usize,
) {
    tokio::spawn(async move {
        let mut pending_requests = pin!(db.get_requests_to_process_stream());
        let fulfiller_signer = NetworkSigner::local(&network_private_key).unwrap();
        let fulfiller_signer = Arc::new(fulfiller_signer);
        let (tx, rx) = crossbeam::channel::unbounded::<PendingRequest>();

        for gpu_id in 0..worker_count {
            let db = db.clone();
            let rx = rx.clone();
            let fulfiller_signer = fulfiller_signer.clone();
            let network_rpc_url = network_rpc_url.clone();
            let programs_s3_region = programs_s3_region.clone();
            let hostname = hostname.clone();

            tokio::spawn(async move {
                while let Ok(pending_request) = rx.recv() {
                    db.update_request(pending_request.id, |r| {
                        r.fulfillment_status = FulfillmentStatus::Assigned;
                    })
                    .await;

                    tracing::debug!("Get program {}", pending_request.vk_hash);
                    let pk = db.get_proving_key(pending_request.vk_hash).await;

                    #[cfg(not(feature = "mock"))]
                    let fulfiller = Fulfiller::new(
                        pk,
                        pending_request.clone(),
                        gpu_id,
                        db.clone(),
                        fulfiller_signer.clone(),
                        network_rpc_url.clone(),
                        programs_s3_region.clone(),
                        hostname.clone(),
                    );

                    #[cfg(feature = "mock")]
                    let fulfiller = Fulfiller::mock(
                        pk,
                        pending_request.clone(),
                        db.clone(),
                        fulfiller_signer.clone(),
                        network_rpc_url.clone(),
                        programs_s3_region.clone(),
                        hostname.clone(),
                    );

                    if let Err(err) = fulfiller.process().await {
                        tracing::error!("Error during proving {}: {err}", pending_request.id);
                        db.update_request(pending_request.id, |r| {
                            r.fulfillment_status = FulfillmentStatus::Unfulfillable;
                        })
                        .await;
                    }
                }
            });
        }

        while let Some(request) = pending_requests.next().await {
            tx.send(request).unwrap();
        }
    });
}
