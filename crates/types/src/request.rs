use std::sync::Arc;

use alloy_primitives::B256;
use sp1_core_executor::ExecutionError;
use sp1_sdk::{
    SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin,
    private::proto::{ProofMode, RequestProofRequestBody},
};
use thiserror::Error;

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
        body: RequestProofRequestBody,
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
