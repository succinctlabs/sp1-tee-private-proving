use std::sync::Arc;

use alloy_primitives::{Address, B256};
use anyhow::Result;
use sp1_sdk::{
    NetworkSigner,
    network::proto::types::{
        CreateProgramRequest, CreateProgramResponse, GetNonceRequest, GetNonceResponse,
        GetProgramRequest, GetProgramResponse, GetProofRequestDetailsRequest,
        GetProofRequestStatusRequest, GetProofRequestStatusResponse, ProofRequest,
        RequestProofRequest, RequestProofResponse, RequestProofResponseBody,
    },
};
use sp1_tee_private_types::prover_network_server::ProverNetwork;
use sp1_tee_private_utils::prover_network_client;
use tonic::{Request, Response, Status};

use crate::db::Db;

#[derive(Debug, Clone)]
pub struct DefaultPrivateProverServer<DB: Db> {
    hostname: String,
    network_rpc_url: String,
    fulfiller_address: Address,
    artifacts_port: u16,
    db: Arc<DB>,
}

impl<DB: Db> DefaultPrivateProverServer<DB> {
    pub fn new(
        hostname: String,
        network_rpc_url: String,
        fulfiller_private_key: String,
        artifacts_port: u16,
        db: Arc<DB>,
    ) -> Self {
        let fulfiller_signer = NetworkSigner::local(&fulfiller_private_key).unwrap();

        Self {
            hostname,
            network_rpc_url,
            fulfiller_address: fulfiller_signer.address(),
            artifacts_port,
            db,
        }
    }
}

#[tonic::async_trait]
impl<DB: Db> ProverNetwork for DefaultPrivateProverServer<DB> {
    /// Proxy CreateProgram requests to the prover network, as the programs need to be registered in order to be able
    /// to send proof request to the prover network.
    async fn create_program(
        &self,
        request: Request<CreateProgramRequest>,
    ) -> Result<Response<CreateProgramResponse>, Status> {
        let request = request.into_inner();
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();
        let response_from_network = network_client.create_program(request).await?;

        Ok(response_from_network)
    }

    /// Proxy GetProgram requests to the prover network.
    async fn get_program(
        &self,
        request: Request<GetProgramRequest>,
    ) -> Result<Response<GetProgramResponse>, Status> {
        let request = request.into_inner();
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        network_client.get_program(request).await
    }

    /// Proxy GeNonce requests to the prover network.
    async fn get_nonce(
        &self,
        request: Request<GetNonceRequest>,
    ) -> Result<Response<GetNonceResponse>, Status> {
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        network_client.get_nonce(request).await
    }

    /// Proxy RequestProof requests to the prover network.
    /// Also inserts them to a queue to be executed and proved by the enclave.
    /// The requests sent to the prover network are associated to a *fake* fulfiller,
    /// and their fulfillment status are updated by the enclave.
    async fn request_proof(
        &self,
        request: Request<RequestProofRequest>,
    ) -> Result<Response<RequestProofResponse>, Status> {
        tracing::debug!("Start request proof");
        let request = request.into_inner();
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        tracing::debug!("Forwarding proof request to the network");
        let response_from_network = network_client.request_proof(request).await?.into_inner();
        let response_body = response_from_network
            .body
            .clone()
            .ok_or_else(|| Status::invalid_argument("missing network response body"))?;

        tracing::debug!("Get proof request details");
        match network_client
            .get_proof_request_details(GetProofRequestDetailsRequest {
                request_id: response_body.request_id.clone(),
            })
            .await?
            .into_inner()
            .request
        {
            Some(mut proof_request) => {
                let request_id = B256::from_slice(&proof_request.request_id);

                // Override stdin URL
                proof_request.stdin_uri = proof_request.stdin_uri.replace(
                    &self.hostname,
                    #[cfg(feature = "local")]
                    &format!("http://localhost:{}", self.artifacts_port),
                    #[cfg(not(feature = "local"))]
                    &format!("http://server:{}", self.artifacts_port),
                );

                if let Some(fulfiller) = &proof_request.fulfiller
                    && fulfiller == self.fulfiller_address.as_slice()
                {
                    tracing::debug!(?request_id, "Insert proof request");
                    self.db.insert_request(proof_request).await;
                } else {
                    tracing::debug!(
                        ?request_id,
                        "Proof request associated with another fulfiller"
                    );
                }
            }
            None => {
                return Err(Status::not_found(
                    "Proof request not present in the network",
                ));
            }
        };

        let response = RequestProofResponse {
            tx_hash: response_from_network.tx_hash.clone(),
            body: Some(RequestProofResponseBody {
                request_id: response_body.request_id.clone(),
            }),
        };

        Ok(Response::new(response))
    }

    async fn take_next_proof_request(
        &self,
        _: Request<()>,
    ) -> Result<Response<ProofRequest>, Status> {
        self.db
            .pop_request()
            .await
            .map(Response::new)
            .ok_or_else(|| Status::not_found("No proof requests in the queue"))
    }

    // Retrieve the proof request status from the enclave DB.
    async fn get_proof_request_status(
        &self,
        request: Request<GetProofRequestStatusRequest>,
    ) -> Result<Response<GetProofRequestStatusResponse>, Status> {
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        network_client.get_proof_request_status(request).await
    }
}
