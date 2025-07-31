use std::{collections::VecDeque, num::NonZeroUsize, sync::Arc};

use alloy_primitives::B256;
use async_stream::stream;
use futures::Stream;
use lru::LruCache;
use sp1_sdk::{SP1ProofWithPublicValues, SP1ProvingKey};
use sp1_tee_private_types::{PendingRequest, Request, UnfulfillableRequestReason};
use tokio::sync::{Mutex, Notify};
use tonic::async_trait;

use crate::db::Db;

#[derive(Debug)]
pub struct InMemoryDb {
    programs: Mutex<LruCache<B256, Arc<SP1ProvingKey>>>,
    pending_requests: Mutex<VecDeque<PendingRequest>>,
    requests: Mutex<LruCache<Vec<u8>, Arc<Request>>>,
    notify_new_pending_request: Notify,
}

impl InMemoryDb {
    pub fn new() -> Self {
        Self {
            programs: Mutex::new(LruCache::new(NonZeroUsize::new(16).unwrap())),
            pending_requests: Mutex::new(VecDeque::new()),
            requests: Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())),
            notify_new_pending_request: Notify::new(),
        }
    }
}

#[async_trait]
impl Db for InMemoryDb {
    async fn insert_program(&self, vk_hash: B256, pk: SP1ProvingKey) {
        let mut programs = self.programs.lock().await;

        programs.push(vk_hash, Arc::new(pk));
    }

    async fn get_program(&self, vk_hash: B256) -> Option<Arc<SP1ProvingKey>> {
        let mut programs = self.programs.lock().await;

        programs.get(&vk_hash).cloned()
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

    async fn set_request_as_assigned(&self, request_id: Vec<u8>) {
        let mut requests = self.requests.lock().await;

        requests.push(request_id, Arc::new(Request::Assigned));
    }

    async fn set_request_as_fulfilled(&self, request_id: Vec<u8>, proof: SP1ProofWithPublicValues) {
        let mut requests = self.requests.lock().await;

        requests.push(
            request_id,
            Arc::new(Request::Fulfilled {
                proof: Arc::new(proof),
            }),
        );
    }

    async fn set_request_as_unfulfillable(
        &self,
        request_id: Vec<u8>,
        reason: UnfulfillableRequestReason,
    ) {
        let mut requests = self.requests.lock().await;

        requests.push(request_id, Arc::new(Request::Unfulfillable { reason }));
    }

    async fn get_proof(&self, request_id: &[u8]) -> Option<Arc<SP1ProofWithPublicValues>> {
        let mut requests = self.requests.lock().await;

        requests.get(request_id).and_then(|r| match r.as_ref() {
            Request::Fulfilled { proof } => Some(proof.clone()),
            _ => None,
        })
    }
}
