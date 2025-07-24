use std::{
    borrow::Cow,
    pin::{Pin, pin},
    sync::Arc,
};

use alloy_primitives::B256;
use anyhow::Result;
use futures::{Stream, StreamExt};
use sp1_sdk::{
    network::proto::types::FulfillmentStatus,
    private::{
        proto::{
            Chunk, CreateProgramResponse, GetProofRequestStatusRequest, ProgramExistsRequest,
            ProgramExistsResponse, RequestProofResponse, RequestProofResponseBody,
            private_prover_server::PrivateProver,
        },
        types::{
            CreateProgramRequestBody, GetProofRequestStatusResponse, RequestProofRequestBody,
            SignedMessage,
        },
        utils::{consume_chunk_stream, into_chunk_stream},
    },
};
use sp1_tee_private_types::{PendingRequest, Request as ProofRequest};
use tonic::{Request, Response, Status, Streaming};
use tracing::instrument;

use crate::{db::Db, fulfiller::Fulfiller};

#[derive(Debug, Clone)]
pub struct DefaultPrivateProverServer<DB: Db> {
    db: Arc<DB>,
}

impl<DB: Db> DefaultPrivateProverServer<DB> {
    pub fn new(db: DB) -> Self {
        let db = Arc::new(db);

        spawn_dispatcher(db.clone());

        Self { db }
    }
}

#[tonic::async_trait]
impl<DB: Db> PrivateProver for DefaultPrivateProverServer<DB> {
    type GetProofRequestStatusStream = Pin<Box<dyn Stream<Item = Result<Chunk, Status>> + Send>>;

    #[instrument(level = "debug", skip_all)]
    async fn create_program(
        &self,
        request: Request<Streaming<Chunk>>,
    ) -> Result<Response<CreateProgramResponse>, Status> {
        let (encoded_request, _) = consume_chunk_stream(request.into_inner())
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        let request = bincode::deserialize::<SignedMessage>(&encoded_request)
            .map_err(|_| Status::invalid_argument("invalid request"))?;
        let body = bincode::deserialize::<CreateProgramRequestBody>(&request.message)
            .map_err(|_| Status::invalid_argument("invalid request"))?;

        self.db
            .insert_program(body.vk_hash, body.pk.into_owned())
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
        request: Request<Streaming<Chunk>>,
    ) -> Result<Response<RequestProofResponse>, Status> {
        tracing::debug!("Start request_proof");
        let (encoded_request, _) = consume_chunk_stream(request.into_inner())
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        tracing::debug!("deserialize message");
        let request = bincode::deserialize::<SignedMessage>(&encoded_request)
            .map_err(|_| Status::invalid_argument("invalid request"))?;

        tracing::debug!("deserialize body");
        let body = bincode::deserialize::<RequestProofRequestBody>(&request.message)
            .map_err(|_| Status::invalid_argument("invalid request"))?;
        let request = PendingRequest::from(body);
        let response = RequestProofResponse {
            tx_hash: vec![],
            body: Some(RequestProofResponseBody {
                request_id: request.id.clone(),
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
    ) -> Result<Response<Self::GetProofRequestStatusStream>, Status> {
        let request_id = request.into_inner().request_id;
        let request = self
            .db
            .get_request(&request_id)
            .await
            .ok_or_else(|| Status::not_found("The request hasn't been requested"))?;

        let proof = self.db.get_proof(&request_id).await;

        let fulfillment_status = match request.as_ref() {
            ProofRequest::Assigned(_) => FulfillmentStatus::Assigned,
            ProofRequest::Fulfilled(_) => FulfillmentStatus::Fulfilled,
            ProofRequest::Unfulfillable(_) => FulfillmentStatus::Unfulfillable,
        };

        let response = GetProofRequestStatusResponse {
            fulfillment_status,
            deadline: request.deadline(),
            proof,
        };

        tracing::debug!("Build stream");
        let stream = into_chunk_stream(&response)
            .map_err(|err| Status::internal(err.to_string()))?
            .map(Ok);

        tracing::debug!("Send status");
        Ok(Response::new(Box::pin(stream)))
    }
}

fn spawn_dispatcher<DB: Db>(db: Arc<DB>) {
    tokio::spawn(async move {
        let mut pending_requests = pin!(db.get_requests_to_process_stream());

        while let Some(request) = pending_requests.next().await {
            let pk = db.get_program(request.vk_hash).await;

            if let Some(pk) = pk {
                let fulfiller = Fulfiller::mock(pk, request.clone());

                match fulfiller.process() {
                    Ok(proof) => {
                        tracing::info!("Proved {}", B256::from_slice(&request.id));
                        db.insert_proof(request.id, proof, request.deadline).await;
                    }
                    Err(err) => {
                        tracing::error!("Failed to prove: {err}");
                    }
                }
            }
        }
    });
}
