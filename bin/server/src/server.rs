use std::{pin::pin, sync::Arc};

use alloy_primitives::B256;
use anyhow::Result;
use futures::StreamExt;
use sp1_sdk::{
    network::proto::types::FulfillmentStatus,
    private::proto::{
        CreateProgramRequest, CreateProgramResponse, GetProofRequestStatusRequest,
        GetProofRequestStatusResponse, ProgramExistsRequest, ProgramExistsResponse, ProofMode,
        RequestProofRequest, RequestProofResponse, RequestProofResponseBody,
        private_prover_server::PrivateProver,
    },
};
use sp1_tee_private_types::{ArtifactType, Key, PendingRequest, Request as ProofRequest};
use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::{
    db::{ArtifactId, Db},
    fulfiller::Fulfiller,
    utils::PresignedUrl,
};

#[derive(Debug, Clone)]
pub struct DefaultPrivateProverServer<DB: Db> {
    hostname: String,
    db: Arc<DB>,
}

impl<DB: Db> DefaultPrivateProverServer<DB> {
    pub fn new(hostname: String, db: Arc<DB>, worker_count: usize) -> Self {
        spawn_workers(db.clone(), worker_count);

        Self { hostname, db }
    }
}

#[tonic::async_trait]
impl<DB: Db> PrivateProver for DefaultPrivateProverServer<DB> {
    #[instrument(level = "debug", skip_all)]
    async fn create_program(
        &self,
        request: Request<CreateProgramRequest>,
    ) -> Result<Response<CreateProgramResponse>, Status> {
        let request = request.into_inner();
        let body = request
            .body
            .ok_or_else(|| Status::invalid_argument("missing body"))?;

        self.db
            .update_artifact_id(
                Key::from_uri(&body.program_uri),
                ArtifactId::VkHash(B256::from_slice(&body.vk_hash)),
            )
            .await;

        Ok(Response::new(CreateProgramResponse {}))
    }

    #[instrument(level = "debug", skip_all)]
    async fn program_exists(
        &self,
        request: Request<ProgramExistsRequest>,
    ) -> Result<Response<ProgramExistsResponse>, Status> {
        let vk_hash = request.into_inner().vk_hash;
        let exists = self
            .db
            .get_program(B256::from_slice(&vk_hash))
            .await
            .is_some();

        Ok(Response::new(ProgramExistsResponse { exists }))
    }

    #[instrument(level = "debug", skip_all)]
    async fn request_proof(
        &self,
        request: Request<RequestProofRequest>,
    ) -> Result<Response<RequestProofResponse>, Status> {
        tracing::debug!("Start request_proof");
        let request = request.into_inner();
        let body = request
            .body
            .ok_or_else(|| Status::invalid_argument("missing body"))?;
        let request_id = B256::random(); // TODO: Handle
        let mode = ProofMode::try_from(body.mode)
            .map_err(|_| Status::invalid_argument("missing proof mode"))?;

        self.db
            .update_artifact_id(
                Key::from_uri(&body.stdin_uri),
                ArtifactId::RequestId(request_id),
            )
            .await;

        let inputs = self
            .db
            .get_inputs(request_id)
            .await
            .ok_or_else(|| Status::invalid_argument("missing stdin"))?;

        let request = PendingRequest::from_request_body(body, request_id, mode, inputs);
        let response = RequestProofResponse {
            tx_hash: vec![],
            body: Some(RequestProofResponseBody {
                request_id: request.id.to_vec(),
            }),
        };

        tracing::debug!("Insert request");
        self.db.insert_request(request).await;

        Ok(Response::new(response))
    }

    #[instrument(level = "debug", skip_all)]
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

        let proof_presigned_url = match request.as_ref() {
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
            proof_presigned_url,
        };

        tracing::debug!("Send status");
        Ok(Response::new(response))
    }
}

fn spawn_workers<DB: Db>(db: Arc<DB>, worker_count: usize) {
    tokio::spawn(async move {
        let mut pending_requests = pin!(db.get_requests_to_process_stream());
        let (tx, rx) = crossbeam::channel::unbounded::<PendingRequest>();

        for gpu_id in 0..worker_count {
            let db = db.clone();
            let rx = rx.clone();

            tokio::spawn(async move {
                while let Ok(request) = rx.recv() {
                    db.set_request_as_assigned(request.id).await;

                    let pk = db.get_program(request.vk_hash).await;

                    if let Some(pk) = pk {
                        let fulfiller = Fulfiller::new(pk, request.clone(), gpu_id);

                        match fulfiller.process() {
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
                }
            });
        }

        while let Some(request) = pending_requests.next().await {
            tx.send(request).unwrap();
        }
    });
}
