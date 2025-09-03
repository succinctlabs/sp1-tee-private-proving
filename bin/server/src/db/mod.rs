use std::sync::Arc;

use alloy_primitives::B256;
use futures::Stream;
use sp1_sdk::{ProofFromNetwork, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin};
use tonic::async_trait;

use crate::types::{Key, PendingRequest, Request, UnfulfillableRequestReason};

mod in_memory;
pub use in_memory::InMemoryDb;

#[async_trait]
pub trait Db: Send + Sync + 'static {
    async fn insert_artifact_request(&self, key: Key);

    async fn consume_artifact_request(&self, key: Key) -> bool;

    async fn insert_artifact(&self, key: Key, artifact: Artifact);

    async fn update_artifact_id(&self, key: Key, new_id: ArtifactId);

    async fn insert_proving_key(&self, vk_hash: B256, pk: Arc<SP1ProvingKey>);

    async fn get_proving_key(&self, vk_hash: B256) -> Option<Arc<SP1ProvingKey>>;

    async fn get_inputs(&self, vk_hash: B256) -> Option<Arc<SP1Stdin>>;

    async fn get_proof(&self, key: Key) -> Option<Arc<ProofFromNetwork>>;

    async fn insert_request(&self, request: PendingRequest);

    async fn get_request(&self, id: &[u8]) -> Option<Arc<Request>>;

    fn get_requests_to_process_stream(&self) -> impl Stream<Item = PendingRequest> + Send + Sync;

    async fn set_request_as_assigned(&self, request_id: B256);

    async fn set_request_as_fulfilled(&self, request_id: B256, proof: SP1ProofWithPublicValues);

    async fn set_request_as_unfulfillable(
        &self,
        request_id: B256,
        reason: UnfulfillableRequestReason,
    );
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum ArtifactId {
    Key(Key),
    RequestId(B256),
}

#[derive(Clone)]
pub enum Artifact {
    Inputs(Arc<SP1Stdin>),
    Proof(Arc<ProofFromNetwork>),
}

impl Artifact {
    pub fn as_inputs(&self) -> Option<Arc<SP1Stdin>> {
        match self {
            Artifact::Inputs(stdin) => Some(stdin.clone()),
            _ => None,
        }
    }

    pub fn as_proof(&self) -> Option<Arc<ProofFromNetwork>> {
        match self {
            Artifact::Proof(proof) => Some(proof.clone()),
            _ => None,
        }
    }
}

impl From<SP1Stdin> for Artifact {
    fn from(value: SP1Stdin) -> Self {
        Self::Inputs(Arc::new(value))
    }
}

impl From<&SP1ProofWithPublicValues> for Artifact {
    fn from(value: &SP1ProofWithPublicValues) -> Self {
        let proof = ProofFromNetwork {
            proof: value.proof.clone(),
            public_values: value.public_values.clone(),
            sp1_version: value.sp1_version.clone(),
        };
        Self::Proof(Arc::new(proof))
    }
}
