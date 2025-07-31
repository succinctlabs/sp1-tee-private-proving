use std::sync::Arc;

use alloy_primitives::B256;
use sp1_core_executor::ExecutionError;
use sp1_sdk::{
    SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin, network::proto::types::ProofMode,
    private::types::RequestProofRequestBody,
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub id: Vec<u8>,
    pub vk_hash: B256,
    pub mode: SP1ProofMode,
    pub stdin: SP1Stdin,
    pub cycle_limit: u64,
    pub gas_limit: u64,
    pub deadline: u64,
}

#[derive(Debug, Error)]
pub enum UnfulfillableRequestReason {
    #[error("Program not registered")]
    ProgramNotRegistered,

    #[error("Deadline exceeded")]
    DeadlineExceeded,

    #[error("Gas limit exceeded")]
    GasLimitExceeded,

    #[error("Execution error: {0}")]
    ExecutionError(#[from] ExecutionError),

    #[error("Proving error: {0}")]
    ProvingError(String),
}

#[derive(Debug)]
pub enum Request {
    Assigned,
    Fulfilled {
        proof: Arc<SP1ProofWithPublicValues>,
    },
    Unfulfillable {
        reason: UnfulfillableRequestReason,
    },
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
            cycle_limit: value.cycle_limit,
            gas_limit: value.gas_limit,
            deadline: value.deadline,
        }
    }
}
