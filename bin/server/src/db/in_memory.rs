use std::{
    collections::{HashSet, VecDeque},
    num::NonZeroUsize,
    sync::Arc,
};

use alloy_primitives::B256;
use async_stream::stream;
use futures::Stream;
use lru::LruCache;
use sp1_sdk::{ProofFromNetwork, SP1ProvingKey, SP1Stdin};
use tokio::sync::{Mutex, Notify};
use tonic::async_trait;

use crate::{
    db::{Artifact, Db},
    types::{Key, PendingRequest, Request, UnfulfillableRequestReason},
};

#[derive(Debug)]
pub struct InMemoryDb {
    artifact_requests: Mutex<HashSet<Key>>,
    artifacts: Mutex<LruCache<Key, Artifact>>,
    proving_keys: Mutex<LruCache<B256, Arc<SP1ProvingKey>>>,
    pending_requests: Mutex<VecDeque<PendingRequest>>,
    requests: Mutex<LruCache<B256, Arc<Request>>>,
    notify_new_pending_request: Notify,
}

impl InMemoryDb {
    pub fn new() -> Self {
        Self {
            artifact_requests: Mutex::new(HashSet::new()),
            artifacts: Mutex::new(LruCache::new(NonZeroUsize::new(1024).unwrap())),
            proving_keys: Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())),
            pending_requests: Mutex::new(VecDeque::new()),
            requests: Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())),
            notify_new_pending_request: Notify::new(),
        }
    }
}

#[async_trait]
impl Db for InMemoryDb {
    async fn insert_artifact_request(&self, key: Key) {
        let mut artifact_requests = self.artifact_requests.lock().await;

        artifact_requests.insert(key);
    }

    async fn consume_artifact_request(&self, key: Key) -> bool {
        let mut artifact_requests = self.artifact_requests.lock().await;

        artifact_requests.remove(&key)
    }

    async fn insert_artifact(&self, key: Key, artifact: Artifact) {
        let mut artifacts = self.artifacts.lock().await;

        artifacts.push(key, artifact);
    }

    async fn insert_proving_key(&self, vk_hash: B256, pk: Arc<SP1ProvingKey>) {
        let mut proving_keys = self.proving_keys.lock().await;

        proving_keys.push(vk_hash, pk);
    }

    async fn get_proving_key(&self, vk_hash: B256) -> Option<Arc<SP1ProvingKey>> {
        let mut proving_keys = self.proving_keys.lock().await;

        proving_keys.get(&vk_hash).cloned()
    }

    async fn get_stdin(&self, key: Key) -> Option<Arc<SP1Stdin>> {
        let mut artifacts = self.artifacts.lock().await;

        artifacts.get(&key).and_then(|a| a.as_inputs())
    }

    async fn get_proof(&self, key: Key) -> Option<Arc<ProofFromNetwork>> {
        let mut artifacts = self.artifacts.lock().await;

        artifacts.get(&key).and_then(|a| a.as_proof())
    }

    async fn insert_request(&self, request: PendingRequest) {
        let mut pending_requests = self.pending_requests.lock().await;
        pending_requests.push_front(request);
        self.notify_new_pending_request.notify_one();
    }

    async fn get_request(&self, id: &[u8]) -> Option<Arc<Request>> {
        let mut requests = self.requests.lock().await;

        requests.get(id).cloned()
    }

    fn get_requests_to_process_stream(&self) -> impl Stream<Item = PendingRequest> + Send + Sync {
        stream! {
            loop {
                let item = {
                    let mut pending_requests = self.pending_requests.lock().await;
                    pending_requests.pop_front()
                };

                match item {
                    Some(value) => {
                        yield value
                    },
                    None => {
                        // Wait for notification when deque is empty
                        self.notify_new_pending_request.notified().await;
                    }
                }
            }
        }
    }

    async fn set_request_as_assigned(&self, request_id: B256) {
        let mut requests = self.requests.lock().await;

        requests.push(request_id, Arc::new(Request::Assigned));
    }

    async fn set_request_as_fulfilled(&self, request_id: B256, proof_key: Key) {
        let mut requests = self.requests.lock().await;

        requests.push(request_id, Arc::new(Request::Fulfilled { proof_key }));
    }

    async fn set_request_as_unfulfillable(
        &self,
        request_id: B256,
        reason: UnfulfillableRequestReason,
    ) {
        let mut requests = self.requests.lock().await;

        requests.push(request_id, Arc::new(Request::Unfulfillable { reason }));
    }
}
