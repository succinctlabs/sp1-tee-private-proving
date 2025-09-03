use std::sync::Arc;

use anyhow::Result;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{CudaProver, Prover, ProverClient, SP1Context, SP1ProofWithPublicValues};
use sp1_tee_private_types::{PendingRequest, UnfulfillableRequestReason};

use crate::db::{ArtifactId, Db, Program};

pub struct Fulfiller<P: Prover<CpuProverComponents>, DB: Db> {
    program: Arc<Program>,
    request: PendingRequest,
    prover: P,
    db: Arc<DB>,
}

impl<DB: Db> Fulfiller<CudaProver, DB> {
    pub fn new(
        program: Arc<Program>,
        request: PendingRequest,
        device_id: usize,
        db: Arc<DB>,
    ) -> Self {
        let port = 3000 + device_id;
        let prover = ProverClient::builder()
            .cuda()
            .server(&format!("http://moongate:{port}/twirp/"))
            .build();
        Self {
            program,
            request,
            prover,
            db,
        }
    }
}

#[cfg(feature = "mock")]
impl<DB: Db> Fulfiller<sp1_sdk::CpuProver, DB> {
    pub fn mock(program: Arc<Program>, request: PendingRequest, db: Arc<DB>) -> Self {
        let prover = ProverClient::builder().mock().build();
        Self {
            program,
            request,
            prover,
            db,
        }
    }
}

impl<P: Prover<CpuProverComponents>, DB: Db> Fulfiller<P, DB> {
    pub async fn process(self) -> Result<SP1ProofWithPublicValues, UnfulfillableRequestReason> {
        let prover = self.prover.inner();
        let context = SP1Context::builder()
            .max_cycles(self.request.cycle_limit)
            .calculate_gas(true)
            .build();

        let pk = match self.program.as_ref() {
            Program::Elf(elf) => {
                tracing::debug!("Setup {}", self.request.id);
                let (pk, _) = self.prover.setup(elf);
                let db = self.db.clone();

                db.update_artifact(
                    ArtifactId::VkHash(self.request.vk_hash.clone()),
                    pk.clone().into(),
                )
                .await;

                Arc::new(pk)
            }
            Program::ProvingKey(pk) => pk.clone(),
        };

        tracing::debug!("Executing {}", self.request.id);
        let (_, _, report) = prover.execute(&pk.elf, &self.request.stdin, context)?;

        if let Some(used_gas) = report.gas
            && used_gas > self.request.gas_limit
        {
            return Err(UnfulfillableRequestReason::GasLimitExceeded);
        }

        tracing::debug!("Start proving {}", self.request.id);
        self.prover
            .prove(&pk, &self.request.stdin, self.request.mode)
            .map_err(|err| UnfulfillableRequestReason::ProvingError(err.to_string()))
    }
}
