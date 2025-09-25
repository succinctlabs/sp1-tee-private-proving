use std::sync::Arc;

use sp1_sdk::network::proto::base_types::ProofRequest;
use tonic::async_trait;

mod in_memory;
pub use in_memory::InMemoryDb;

#[async_trait]
pub trait Db: Send + Sync + 'static {
    async fn insert_artifact_request(&self, id: String);

    async fn consume_artifact_request(&self, id: String) -> bool;

    async fn insert_stdin(&self, id: String, stdin: Vec<u8>);

    async fn get_stdin(&self, id: &str) -> Option<Arc<Vec<u8>>>;

    async fn insert_request(&self, proof_request: ProofRequest);

    async fn pop_request(&self) -> Option<ProofRequest>;
}
