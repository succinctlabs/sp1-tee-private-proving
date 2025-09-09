use std::sync::Arc;

use alloy_primitives::B256;
use sp1_sdk::{
    SP1ProofMode, SP1Stdin,
    network::proto::types::{
        ExecutionStatus, FulfillmentStatus, ProofMode, RequestProofRequestBody,
    },
};

#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub id: B256,
    pub vk_hash: B256,
    pub mode: SP1ProofMode,
    pub stdin: Arc<SP1Stdin>,
    pub cycle_limit: u64,
    pub gas_limit: u64,
    pub deadline: u64,
}

impl PendingRequest {
    pub fn from_request_body(
        body: &RequestProofRequestBody,
        id: B256,
        mode: ProofMode,
        stdin: Arc<SP1Stdin>,
    ) -> Self {
        let mode = match mode {
            ProofMode::Core => SP1ProofMode::Core,
            ProofMode::Compressed => SP1ProofMode::Compressed,
            ProofMode::Plonk => SP1ProofMode::Plonk,
            ProofMode::Groth16 => SP1ProofMode::Groth16,
            _ => SP1ProofMode::Core,
        };

        PendingRequest {
            id,
            vk_hash: B256::from_slice(&body.vk_hash),
            mode,
            stdin,
            cycle_limit: body.cycle_limit,
            gas_limit: body.gas_limit,
            deadline: body.deadline,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProofRequest {
    pub request_tx_hash: Vec<u8>,
    pub execution_status: ExecutionStatus,
    pub fulfillment_status: FulfillmentStatus,
    pub proof_uri: Option<String>,
    pub fulfill_tx_hash: Option<Vec<u8>>,
}

impl ProofRequest {
    pub fn new(request_tx_hash: Vec<u8>) -> Self {
        Self {
            request_tx_hash,
            execution_status: ExecutionStatus::Unexecuted,
            fulfillment_status: FulfillmentStatus::Requested,
            proof_uri: None,
            fulfill_tx_hash: None,
        }
    }
}
