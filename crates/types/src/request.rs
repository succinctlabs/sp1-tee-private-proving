use std::sync::Arc;

use alloy_primitives::B256;
use sp1_sdk::{
    SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin, network::proto::types::ProofMode,
    private::types::RequestProofRequestBody,
};
#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub id: Vec<u8>,
    pub vk_hash: B256,
    pub mode: SP1ProofMode,
    pub stdin: SP1Stdin,
    pub deadline: u64,
}

#[derive(Debug, Clone)]
pub struct AssignedRequest {
    pub deadline: u64,
}

#[derive(Debug, Clone)]
pub struct FulfilledRequest {
    pub deadline: u64,
    pub proof: Arc<SP1ProofWithPublicValues>,
}

#[derive(Debug, Clone)]
pub struct UnfulfillableRequest {
    pub deadline: u64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum Request {
    Assigned(AssignedRequest),
    Fulfilled(FulfilledRequest),
    Unfulfillable(UnfulfillableRequest),
}

impl Request {
    pub fn deadline(&self) -> u64 {
        match self {
            Request::Assigned(assigned_request) => assigned_request.deadline,
            Request::Fulfilled(fulfilled_request) => fulfilled_request.deadline,
            Request::Unfulfillable(unfulfillable_request) => unfulfillable_request.deadline,
        }
    }
}

impl<'a> From<RequestProofRequestBody<'a>> for PendingRequest {
    fn from(value: RequestProofRequestBody) -> Self {
        let mode = match value.mode {
            ProofMode::Core => SP1ProofMode::Core,
            ProofMode::Compressed => SP1ProofMode::Compressed,
            ProofMode::Plonk => SP1ProofMode::Plonk,
            ProofMode::Groth16 => SP1ProofMode::Groth16,
            _ => SP1ProofMode::Core,
        };

        PendingRequest {
            id: B256::random().to_vec(), // TODO
            vk_hash: value.vk_hash,
            mode,
            stdin: value.stdin.into_owned(),
            deadline: value.deadline,
        }
    }
}
