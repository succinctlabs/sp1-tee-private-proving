use std::{
    collections::{HashSet, VecDeque},
    num::NonZeroUsize,
    sync::Arc,
};

use lru::LruCache;
use sp1_sdk::network::proto::types::ProofRequest;
use tokio::sync::Mutex;
use tonic::async_trait;

use crate::db::Db;

#[derive(Debug)]
pub struct InMemoryDb {
    artifact_requests: Mutex<HashSet<String>>,
    stdins: Mutex<LruCache<String, Arc<Vec<u8>>>>,
    proof_requests: Mutex<VecDeque<ProofRequest>>,
}

impl InMemoryDb {
    pub fn new() -> Self {
        Self {
            artifact_requests: Mutex::new(HashSet::new()),
            stdins: Mutex::new(LruCache::new(NonZeroUsize::new(1024).unwrap())),
            proof_requests: Mutex::new(VecDeque::new()),
        }
    }
}

#[async_trait]
impl Db for InMemoryDb {
    async fn insert_artifact_request(&self, id: String) {
        let mut artifact_requests = self.artifact_requests.lock().await;

        artifact_requests.insert(id);
    }

    async fn consume_artifact_request(&self, id: String) -> bool {
        let mut artifact_requests = self.artifact_requests.lock().await;

        artifact_requests.remove(&id)
    }

    async fn insert_stdin(&self, id: String, stdin: Vec<u8>) {
        let mut stdins = self.stdins.lock().await;

        stdins.push(id, Arc::new(stdin));
    }

    async fn get_stdin(&self, id: &str) -> Option<Arc<Vec<u8>>> {
        let mut stdins = self.stdins.lock().await;

        stdins.get(id).cloned()
    }

    async fn insert_request(&self, proof_request: ProofRequest) {
        let mut proof_requests = self.proof_requests.lock().await;

        proof_requests.push_back(proof_request);
    }

    async fn pop_request(&self) -> Option<ProofRequest> {
        let mut proof_requests = self.proof_requests.lock().await;

        proof_requests.pop_front()
    }
}
