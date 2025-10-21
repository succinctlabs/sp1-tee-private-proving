use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use lru::LruCache;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{
    CudaProver, NetworkSigner, Prover, ProverClient, SP1Context, SP1ProofMode, SP1Prover,
    SP1ProvingKey, SP1Stdin,
    network::{
        B256,
        proto::base_types::{
            ExecutionStatus, FailFulfillmentRequest, FailFulfillmentRequestBody,
            FulfillProofRequest, FulfillProofRequestBody, GetNonceRequest, GetProgramRequest,
            MessageFormat, ProofMode, ProofRequest,
        },
    },
};
use sp1_tee_private_utils::{Signable, private_network_client, prover_network_client};
use spn_artifacts::{Artifact, extract_artifact_name};
use tokio::{
    sync::Mutex,
    time::{Instant, sleep},
};
use tonic::Code;

const REFRESH_INTERVAL_SEC: u64 = 3;

pub async fn run(
    network_rpc_url: String,
    private_server_rpc_url: String,
    fulfiller_private_key: String,
    programs_s3_region: String,
    worker_count: usize,
) -> anyhow::Result<()> {
    let proving_keys = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(32).unwrap())));
    let fulfiller_signer = NetworkSigner::local(&fulfiller_private_key)?;
    let fulfiller_signer = Arc::new(fulfiller_signer);
    let private_client = private_network_client(&private_server_rpc_url).await?;

    for gpu_id in 0..worker_count {
        let proving_keys = proving_keys.clone();
        let fulfiller_signer = fulfiller_signer.clone();
        let network_rpc_url = network_rpc_url.clone();
        let programs_s3_region = programs_s3_region.clone();
        let mut private_client = private_client.clone();

        tokio::spawn(async move {
            loop {
                match private_client.take_next_proof_request(()).await {
                    Ok(proof_request) => {
                        let proof_request = proof_request.into_inner();
                        let request_id = B256::from_slice(&proof_request.request_id);

                        #[cfg(not(feature = "cpu"))]
                        let fulfiller = Fulfiller::new(
                            proof_request,
                            gpu_id,
                            proving_keys.clone(),
                            fulfiller_signer.clone(),
                            network_rpc_url.clone(),
                            programs_s3_region.clone(),
                        );

                        #[cfg(feature = "cpu")]
                        let fulfiller = Fulfiller::mock(
                            proof_request,
                            proving_keys.clone(),
                            fulfiller_signer.clone(),
                            network_rpc_url.clone(),
                            programs_s3_region.clone(),
                        );

                        if let Err(err) = fulfiller.process().await {
                            tracing::error!(?request_id, "Error during proving: {err}");
                        } else {
                            tracing::info!(?request_id, "Proving sucessful!");
                        }
                    }
                    Err(status) => {
                        if status.code() != Code::NotFound {
                            tracing::error!("{}", status.message());
                        } else {
                            tracing::debug!("{}", status.message());
                        }
                    }
                }

                // Wait for the next interval.
                sleep(Duration::from_secs(REFRESH_INTERVAL_SEC)).await;
            }
        });
    }

    Ok(())
}

pub struct Fulfiller<P: Prover<CpuProverComponents>> {
    proof_request: ProofRequest,
    prover: P,
    proving_keys: Arc<Mutex<LruCache<Vec<u8>, Arc<SP1ProvingKey>>>>,
    fulfiller_signer: Arc<NetworkSigner>,
    network_rpc_url: String,
    programs_s3_region: String,
}

impl Fulfiller<CudaProver> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proof_request: ProofRequest,
        device_id: usize,
        proving_keys: Arc<Mutex<LruCache<Vec<u8>, Arc<SP1ProvingKey>>>>,
        fulfiller_signer: Arc<NetworkSigner>,
        network_rpc_url: String,
        programs_s3_region: String,
    ) -> Self {
        let port = 3000 + device_id;
        let prover = ProverClient::builder()
            .cuda()
            .server(&format!("http://moongate:{port}/twirp/"))
            .build();

        Self {
            proof_request,
            prover,
            proving_keys,
            fulfiller_signer,
            network_rpc_url,
            programs_s3_region,
        }
    }
}

#[cfg(feature = "cpu")]
impl Fulfiller<sp1_sdk::CpuProver> {
    pub fn cpu(
        proof_request: ProofRequest,
        proving_keys: Arc<Mutex<LruCache<Vec<u8>, Arc<SP1ProvingKey>>>>,
        fulfiller_signer: Arc<NetworkSigner>,
        network_rpc_url: String,
        programs_s3_region: String,
    ) -> Self {
        let prover = ProverClient::builder().cpu().build();
        Self {
            proof_request,
            prover,
            proving_keys,
            fulfiller_signer,
            network_rpc_url,
            programs_s3_region,
        }
    }
}

impl<P: Prover<CpuProverComponents>> Fulfiller<P> {
    pub async fn process(self) -> Result<()> {
        let request_id = B256::from_slice(&self.proof_request.request_id);
        let prover = self.prover.inner();
        let context = SP1Context::builder()
            .max_cycles(self.proof_request.cycle_limit)
            .calculate_gas(true)
            .build();
        let mut network_client = prover_network_client(&self.network_rpc_url).await?;

        let pk = {
            self.proving_keys
                .lock()
                .await
                .get(&self.proof_request.vk_hash)
                .cloned()
        };

        let pk = match pk {
            Some(pk) => pk,
            None => {
                // If the pk is not cached in the DB, retrieve the elf from the prover network,
                // call setup(), and insert the pk to the cacghe.
                tracing::debug!(?request_id, "Setup");

                let program = network_client
                    .get_program(GetProgramRequest {
                        vk_hash: self.proof_request.vk_hash.to_vec(),
                    })
                    .await?
                    .into_inner()
                    .program
                    .ok_or(anyhow!("Program not registered"))?;
                let artifact = Artifact {
                    id: extract_artifact_name(&program.program_uri)?,
                    label: String::from(""),
                    expiry: None,
                };

                let elf = artifact
                    .download_program_from_uri::<Vec<u8>>(
                        &program.program_uri,
                        &self.programs_s3_region,
                    )
                    .await?;

                let (pk, _) = self.prover.setup(&elf);
                let pk = Arc::new(pk);

                self.proving_keys
                    .lock()
                    .await
                    .push(self.proof_request.vk_hash.clone(), pk.clone());

                pk
            }
        };

        let stdin = retrieve_stdin(&self.proof_request.stdin_uri).await?;
        let proof_mode = ProofMode::try_from(self.proof_request.mode)?;
        let proof_mode = match proof_mode {
            ProofMode::Core => SP1ProofMode::Core,
            ProofMode::Compressed => SP1ProofMode::Compressed,
            ProofMode::Plonk => SP1ProofMode::Plonk,
            ProofMode::Groth16 => SP1ProofMode::Groth16,
            _ => SP1ProofMode::Core,
        };

        tracing::debug!(?request_id, "Executing");
        let (execution_result, _, gas_used) = execute_program(&pk.elf, &stdin, prover, context);

        let (execution_result, _) = match execution_result {
            Ok(_) => {
                if let Some(gas_used) = gas_used
                    && gas_used > self.proof_request.gas_limit
                {
                    (
                        Err(anyhow!("Gas limit excedeed")),
                        ExecutionStatus::Unexecutable,
                    )
                } else {
                    (Ok(()), ExecutionStatus::Executed)
                }
            }
            Err(err) => (Err(anyhow!("{err}")), ExecutionStatus::Unexecutable),
        };

        // Return early if the execution failed
        if let Err(err) = execution_result {
            bail!("{err}")
        };

        tracing::debug!(?request_id, "Start proving");
        let prove_start = Instant::now();
        let proof = self.prover.prove(&pk, &stdin, proof_mode);
        let prove_duration = prove_start.elapsed();

        let nonce = network_client
            .get_nonce(GetNonceRequest {
                address: self.fulfiller_signer.address().to_vec(),
            })
            .await?
            .into_inner();

        match proof {
            Ok(proof) => {
                tracing::info!(
                    ?request_id,
                    "Proof generated in {}s",
                    prove_duration.as_secs_f64()
                );
                let encoded_proof = bincode::serialize(&proof)?;

                let body = FulfillProofRequestBody {
                    nonce: nonce.nonce,
                    request_id: self.proof_request.request_id.clone(),
                    proof: encoded_proof,
                    reserved_metadata: None,
                };

                // fulfill the proof on the prover network
                network_client
                    .fulfill_proof(FulfillProofRequest {
                        format: MessageFormat::Binary.into(),
                        signature: body.sign(&self.fulfiller_signer).await?,
                        body: Some(body),
                    })
                    .await
                    .map_err(|err| anyhow!("Failed to fulfill: {err}"))?;

                tracing::debug!(?request_id, "Proof fullfilled");
            }
            Err(err) => {
                tracing::error!(?request_id, "Failed to prove: {err}");

                let body = FailFulfillmentRequestBody {
                    nonce: nonce.nonce,
                    request_id: self.proof_request.request_id.clone(),
                    error: None,
                };

                // Set the proof as unfulfillable on the prover network
                network_client
                    .fail_fulfillment(FailFulfillmentRequest {
                        format: MessageFormat::Binary.into(),
                        signature: body.sign(&self.fulfiller_signer).await?,
                        body: Some(body),
                    })
                    .await?;

                tracing::debug!(?request_id, "Proof marked as unfulfillable");
            }
        }

        Ok(())
    }
}

async fn retrieve_stdin(stdin_uri: &str) -> Result<SP1Stdin> {
    tracing::debug!("Download {stdin_uri}");

    let client = reqwest::Client::new();
    let res = client
        .get(stdin_uri)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .context("Failed to GET HTTPS URL")?;

    if !res.status().is_success() {
        return Err(anyhow!(
            "Failed to download from HTTPS URL {stdin_uri}: status {}",
            res.status()
        ));
    }
    let bytes = res
        .bytes()
        .await
        .context("Failed to read HTTPS response body")?;

    let stdin = bincode::deserialize(&bytes).context("Failed to deserialize stdin")?;

    Ok(stdin)
}

fn execute_program<'a>(
    elf: &[u8],
    stdin: &SP1Stdin,
    prover: &'a SP1Prover,
    context: SP1Context<'a>,
) -> (Result<()>, Option<u64>, Option<u64>) {
    match prover.execute(elf, stdin, context) {
        Ok((_, _, report)) => (Ok(()), Some(report.total_instruction_count()), report.gas),
        Err(err) => (Err(anyhow!("{err}")), None, None),
    }
}
