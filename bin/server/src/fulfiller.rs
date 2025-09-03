use std::sync::Arc;

use anyhow::Result;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{
    CudaProver, Prover, ProverClient, SP1Context, SP1ProofWithPublicValues, SP1ProvingKey,
    network::NetworkClient,
};
use spn_artifacts::{Artifact, extract_artifact_name};

use crate::{
    db::Db,
    types::{PendingRequest, UnfulfillableRequestReason},
};

pub struct Fulfiller<P: Prover<CpuProverComponents>, DB: Db> {
    pk: Option<Arc<SP1ProvingKey>>,
    request: PendingRequest,
    prover: P,
    db: Arc<DB>,
    network_client: Arc<NetworkClient>,
    programs_s3_region: String,
}

impl<DB: Db> Fulfiller<CudaProver, DB> {
    pub fn new(
        pk: Option<Arc<SP1ProvingKey>>,
        request: PendingRequest,
        device_id: usize,
        db: Arc<DB>,
        network_client: Arc<NetworkClient>,
        programs_s3_region: String,
    ) -> Self {
        let port = 3000 + device_id;
        let prover = ProverClient::builder()
            .cuda()
            .server(&format!("http://moongate:{port}/twirp/"))
            .build();

        Self {
            pk,
            request,
            prover,
            db,
            network_client,
            programs_s3_region,
        }
    }
}

#[cfg(feature = "mock")]
impl<DB: Db> Fulfiller<sp1_sdk::CpuProver, DB> {
    pub fn mock(
        pk: Option<Arc<SP1ProvingKey>>,
        request: PendingRequest,
        db: Arc<DB>,
        network_client: Arc<NetworkClient>,
        programs_s3_region: String,
    ) -> Self {
        let prover = ProverClient::builder().mock().build();
        Self {
            pk,
            request,
            prover,
            db,
            network_client,
            programs_s3_region,
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

        let pk = match self.pk {
            Some(pk) => pk,
            None => {
                tracing::debug!("Setup {}", self.request.id);

                let program = self
                    .network_client
                    .get_program(self.request.vk_hash)
                    .await
                    .map_err(|err| UnfulfillableRequestReason::Other(err.to_string()))?
                    .ok_or(UnfulfillableRequestReason::ProgramNotFound)?
                    .program
                    .ok_or(UnfulfillableRequestReason::ProgramNotRegistered)?;
                let artifact = Artifact {
                    id: extract_artifact_name(&program.program_uri).unwrap(),
                    label: String::from(""),
                    expiry: None,
                };

                let elf = artifact
                    .download_program_from_uri::<Vec<u8>>(
                        &program.program_uri,
                        &self.programs_s3_region,
                    )
                    .await
                    .unwrap();

                let (pk, _) = self.prover.setup(&elf);
                let pk = Arc::new(pk);
                let db = self.db.clone();

                db.insert_proving_key(self.request.vk_hash.clone(), pk.clone())
                    .await;

                pk
            }
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
