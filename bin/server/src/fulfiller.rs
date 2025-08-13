use std::sync::Arc;

use anyhow::Result;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{
    CpuProver, CudaProver, Prover, ProverClient, SP1Context, SP1ProofWithPublicValues,
    SP1ProvingKey,
};
use sp1_tee_private_types::{PendingRequest, UnfulfillableRequestReason};

pub struct Fulfiller<P: Prover<CpuProverComponents>> {
    pk: Arc<SP1ProvingKey>,
    request: PendingRequest,
    prover: P,
}

impl Fulfiller<CudaProver> {
    pub fn new(pk: Arc<SP1ProvingKey>, request: PendingRequest, device_id: usize) -> Self {
        let port = 3000 + device_id;
        let prover = ProverClient::builder()
            .cuda()
            .server(&format!("http://moongate:{port}/twirp/"))
            .build();
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
    pub fn process(self) -> Result<SP1ProofWithPublicValues, UnfulfillableRequestReason> {
        tracing::debug!("Executing {}", self.request.id);
        let prover = self.prover.inner();
        let context = SP1Context::builder()
            .max_cycles(self.request.cycle_limit)
            .calculate_gas(true)
            .build();

        let (_, _, report) = prover.execute(&self.pk.elf, &self.request.stdin, context)?;

        if let Some(used_gas) = report.gas
            && used_gas > self.request.gas_limit
        {
            return Err(UnfulfillableRequestReason::GasLimitExceeded);
        }

        tracing::debug!("Start proving {}", self.request.id);
        self.prover
            .prove(&self.pk, &self.request.stdin, self.request.mode)
            .map_err(|err| UnfulfillableRequestReason::ProvingError(err.to_string()))
    }
}
