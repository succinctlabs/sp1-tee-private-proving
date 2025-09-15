use std::sync::Arc;

use anyhow::Result;
use sp1_sdk::network::proto::artifact::{
    ArtifactType, CreateArtifactRequest, CreateArtifactResponse,
    artifact_store_client::ArtifactStoreClient, artifact_store_server::ArtifactStore,
};
use sp1_tee_private_utils::{configure_endpoint, generate_id, presigned_url};
use tonic::{Request, Response, Status, transport::Channel};

use crate::db::Db;

pub struct DefaultArtifactStoreServer<DB: Db> {
    hostname: String,
    network_rpc_url: String,
    db: Arc<DB>,
}

impl<DB: Db> DefaultArtifactStoreServer<DB> {
    pub async fn new(hostname: String, network_rpc_url: String, db: Arc<DB>) -> Self {
        Self {
            hostname,
            network_rpc_url,
            db,
        }
    }

    async fn artifact_store_client(&self) -> Result<ArtifactStoreClient<Channel>> {
        let channel = configure_endpoint(&self.network_rpc_url)?.connect().await?;
        Ok(ArtifactStoreClient::new(channel))
    }
}

#[tonic::async_trait]
impl<DB: Db> ArtifactStore for DefaultArtifactStoreServer<DB> {
    async fn create_artifact(
        &self,
        request: Request<CreateArtifactRequest>,
    ) -> Result<Response<CreateArtifactResponse>, Status> {
        // Parse the request.
        let request = request.into_inner();

        let artifact_type = ArtifactType::try_from(request.artifact_type)
            .unwrap_or(ArtifactType::UnspecifiedArtifactType);

        match artifact_type {
            ArtifactType::Program => {
                let mut artifact_store = self.artifact_store_client().await.unwrap();

                artifact_store.create_artifact(request).await
            }
            ArtifactType::Stdin => {
                let id = generate_id();
                let artifact_presigned_url =
                    presigned_url(&self.hostname, ArtifactType::Stdin, &id);

                tracing::info!("created presigned url: {}", artifact_presigned_url);

                self.db.insert_artifact_request(id).await;

                Ok(Response::new(CreateArtifactResponse {
                    artifact_uri: artifact_presigned_url.clone(),
                    artifact_presigned_url,
                }))
            }
            _ => Err(Status::unavailable("")),
        }
    }
}
