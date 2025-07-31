use std::sync::Arc;

use alloy_primitives::B256;
use futures::Stream;
use sp1_sdk::{SP1ProofWithPublicValues, SP1ProvingKey};
use sp1_tee_private_types::{PendingRequest, Request, UnfulfillableRequestReason};
use tonic::async_trait;

mod in_memory;
pub use in_memory::InMemoryDb;

#[async_trait]
pub trait Db: Send + Sync + 'static {
    async fn insert_program(&self, vk_hash: B256, pk: SP1ProvingKey);

    async fn get_program(&self, vk_hash: B256) -> Option<Arc<SP1ProvingKey>>;

    async fn insert_request(&self, request: PendingRequest);

    async fn get_request(&self, id: &[u8]) -> Option<Arc<Request>>;

    fn get_requests_to_process_stream(&self) -> impl Stream<Item = PendingRequest> + Send + Sync;

    async fn set_request_as_assigned(&self, request_id: Vec<u8>);

    async fn set_request_as_fulfilled(&self, request_id: Vec<u8>, proof: SP1ProofWithPublicValues);

    async fn set_request_as_unfulfillable(
        &self,
        request_id: Vec<u8>,
        reason: UnfulfillableRequestReason,
    );

    async fn get_proof(&self, request_id: &[u8]) -> Option<Arc<SP1ProofWithPublicValues>>;
}
