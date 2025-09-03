use std::sync::Arc;

use sp1_sdk::network::proto::artifact::{
    CreateArtifactRequest, CreateArtifactResponse, artifact_store_server::ArtifactStore,
};
use sp1_tee_private_types::ArtifactType;
use tonic::{Request, Response, Status};

use crate::{db::Db, utils::PresignedUrl};

pub struct DefaultArtifactStoreServer<DB: Db> {
    hostname: String,
    db: Arc<DB>,
}

impl<DB: Db> DefaultArtifactStoreServer<DB> {
    pub async fn new(hostname: String, db: Arc<DB>) -> Self {
        Self { hostname, db }
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
        let presigned = PresignedUrl::new(&artifact_type);

        let artifact_presigned_url = presigned.url(&self.hostname);

        tracing::info!("created presigned url: {}", artifact_presigned_url);

        self.db.insert_artifact_request(presigned.key.clone()).await;

        Ok(Response::new(CreateArtifactResponse {
            artifact_uri: presigned.key.as_uri(),
            artifact_presigned_url,
        }))
    }
}
