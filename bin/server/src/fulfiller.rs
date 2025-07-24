use std::sync::Arc;

use alloy_primitives::B256;
use anyhow::Result;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{
    CpuProver, CudaProver, Prover, ProverClient, SP1ProofWithPublicValues, SP1ProvingKey,
};
use sp1_tee_private_types::PendingRequest;

pub struct Fulfiller<P: Prover<CpuProverComponents>> {
    pk: Arc<SP1ProvingKey>,
    request: PendingRequest,
    prover: P,
}

impl Fulfiller<CudaProver> {
    pub fn new(pk: Arc<SP1ProvingKey>, request: PendingRequest) -> Self {
        let prover = ProverClient::builder().cuda().server("endpoint").build();
        Self {
            pk,
            request,
            prover,
        }
    }
}

impl Fulfiller<CpuProver> {
    pub fn mock(pk: Arc<SP1ProvingKey>, request: PendingRequest) -> Self {
        let prover = ProverClient::builder().mock().build();
        Self {
            pk,
            request,
            prover,
        }
    }
}

impl<P: Prover<CpuProverComponents>> Fulfiller<P> {
    pub fn process(self) -> Result<SP1ProofWithPublicValues> {
        tracing::debug!("Start proving {}", B256::from_slice(&self.request.id));
        self.prover
            .prove(&self.pk, &self.request.stdin, self.request.mode)
    }
}
