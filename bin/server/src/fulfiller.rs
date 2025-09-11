use std::{pin::pin, sync::Arc};

use anyhow::{Result, anyhow, bail};
use futures::StreamExt;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{
    CudaProver, NetworkSigner, Prover, ProverClient, SP1Context, SP1Prover, SP1ProvingKey,
    SP1Stdin,
    network::proto::{
        artifact::ArtifactType,
        types::{
            ExecutionStatus, FailFulfillmentRequest, FailFulfillmentRequestBody,
            FulfillProofRequest, FulfillProofRequestBody, FulfillmentStatus, GetNonceRequest,
            GetProgramRequest, MessageFormat,
        },
    },
};
use spn_artifacts::{Artifact, extract_artifact_name};

use crate::{
    db::Db,
    types::{Key, PendingRequest},
    utils::{Signable, prover_network_client},
};

pub fn spawn_workers<DB: Db>(
    db: Arc<DB>,
    network_rpc_url: String,
    network_private_key: String,
    programs_s3_region: String,
    hostname: String,
    worker_count: usize,
) {
    tokio::spawn(async move {
        let mut pending_requests = pin!(db.get_requests_to_process_stream());
        let fulfiller_signer = NetworkSigner::local(&network_private_key).unwrap();
        let fulfiller_signer = Arc::new(fulfiller_signer);
        let (tx, rx) = crossbeam::channel::unbounded::<PendingRequest>();

        for gpu_id in 0..worker_count {
            let db = db.clone();
            let rx = rx.clone();
            let fulfiller_signer = fulfiller_signer.clone();
            let network_rpc_url = network_rpc_url.clone();
            let programs_s3_region = programs_s3_region.clone();
            let hostname = hostname.clone();

            tokio::spawn(async move {
                while let Ok(pending_request) = rx.recv() {
                    db.update_request(pending_request.id, |r| {
                        r.fulfillment_status = FulfillmentStatus::Assigned;
                    })
                    .await;

                    tracing::debug!("Get program {}", pending_request.vk_hash);
                    let pk = db.get_proving_key(pending_request.vk_hash).await;

                    #[cfg(not(feature = "mock"))]
                    let fulfiller = Fulfiller::new(
                        pk,
                        pending_request.clone(),
                        gpu_id,
                        db.clone(),
                        fulfiller_signer.clone(),
                        network_rpc_url.clone(),
                        programs_s3_region.clone(),
                        hostname.clone(),
                    );

                    #[cfg(feature = "mock")]
                    let fulfiller = Fulfiller::mock(
                        pk,
                        pending_request.clone(),
                        db.clone(),
                        fulfiller_signer.clone(),
                        network_rpc_url.clone(),
                        programs_s3_region.clone(),
                        hostname.clone(),
                    );

                    if let Err(err) = fulfiller.process().await {
                        tracing::error!("Error during proving {}: {err}", pending_request.id);
                        db.update_request(pending_request.id, |r| {
                            r.fulfillment_status = FulfillmentStatus::Unfulfillable;
                        })
                        .await;
                    } else {
                        tracing::info!("Proving {} sucessful!", pending_request.id);
                    }
                }
            });
        }

        while let Some(request) = pending_requests.next().await {
            tx.send(request).unwrap();
        }
    });
}

pub struct Fulfiller<P: Prover<CpuProverComponents>, DB: Db> {
    pk: Option<Arc<SP1ProvingKey>>,
    request: PendingRequest,
    prover: P,
    db: Arc<DB>,
    fulfiller_signer: Arc<NetworkSigner>,
    network_rpc_url: String,
    programs_s3_region: String,
    hostname: String,
}

impl<DB: Db> Fulfiller<CudaProver, DB> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pk: Option<Arc<SP1ProvingKey>>,
        request: PendingRequest,
        device_id: usize,
        db: Arc<DB>,
        fulfiller_signer: Arc<NetworkSigner>,
        network_rpc_url: String,
        programs_s3_region: String,
        hostname: String,
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
            fulfiller_signer,
            network_rpc_url,
            programs_s3_region,
            hostname,
        }
    }
}

#[cfg(feature = "mock")]
impl<DB: Db> Fulfiller<sp1_sdk::CpuProver, DB> {
    pub fn mock(
        pk: Option<Arc<SP1ProvingKey>>,
        request: PendingRequest,
        db: Arc<DB>,
        fulfiller_signer: Arc<NetworkSigner>,
        network_rpc_url: String,
        programs_s3_region: String,
        hostname: String,
    ) -> Self {
        let prover = ProverClient::builder().mock().build();
        Self {
            pk,
            request,
            prover,
            db,
            fulfiller_signer,
            network_rpc_url,
            programs_s3_region,
            hostname,
        }
    }
}

impl<P: Prover<CpuProverComponents>, DB: Db> Fulfiller<P, DB> {
    pub async fn process(self) -> Result<()> {
        let prover = self.prover.inner();
        let context = SP1Context::builder()
            .max_cycles(self.request.cycle_limit)
            .calculate_gas(true)
            .build();
        let mut network_client = prover_network_client(&self.network_rpc_url).await.unwrap();

        let pk = match self.pk {
            Some(pk) => pk,
            None => {
                // If the pk is not cached in the DB, retrieve the elf from the prover network,
                // call setup(), and insert the pk to the cacghe.
                tracing::debug!("Setup {}", self.request.id);

                let program = network_client
                    .get_program(GetProgramRequest {
                        vk_hash: self.request.vk_hash.to_vec(),
                    })
                    .await?
                    .into_inner()
                    .program
                    .ok_or(anyhow!("Program not registered"))?;
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

                self.db
                    .insert_proving_key(self.request.vk_hash, pk.clone())
                    .await;

                pk
            }
        };

        tracing::debug!("Executing {}", self.request.id);
        let (execution_result, _, gas_used) =
            execute_program(&pk.elf, &self.request.stdin, prover, context);

        let (execution_result, execution_status) = match execution_result {
            Ok(_) => {
                if let Some(gas_used) = gas_used
                    && gas_used > self.request.gas_limit
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

        self.db
            .update_request(self.request.id, |r| {
                r.execution_status = execution_status;
            })
            .await;

        // Return early if the execution failed
        if let Err(err) = execution_result {
            bail!("{err}")
        };

        tracing::debug!("Start proving {}", self.request.id);
        let proof = self
            .prover
            .prove(&pk, &self.request.stdin, self.request.mode);

        let nonce = network_client
            .get_nonce(GetNonceRequest {
                address: self.fulfiller_signer.address().to_vec(),
            })
            .await?
            .into_inner();

        match proof {
            Ok(proof) => {
                tracing::debug!(?self.request.id, "Proof generated");
                let proof_key = Key::generate(&ArtifactType::Proof);
                let encoded_proof = bincode::serialize(&proof)?;

                self.db
                    .insert_artifact(proof_key.clone(), proof.into())
                    .await;

                let body = FulfillProofRequestBody {
                    nonce: nonce.nonce,
                    request_id: self.request.id.to_vec(),
                    proof: encoded_proof,
                    reserved_metadata: None,
                };

                // fulfill the proof on the prover network
                let fulfill_resp = network_client
                    .fulfill_proof(FulfillProofRequest {
                        format: MessageFormat::Binary.into(),
                        signature: body.sign(&self.fulfiller_signer).await?,
                        body: Some(body),
                    })
                    .await?
                    .into_inner();

                self.db
                    .update_request(self.request.id, |r| {
                        r.fulfillment_status = FulfillmentStatus::Fulfilled;
                        r.fulfill_tx_hash = Some(fulfill_resp.tx_hash.clone());
                        r.proof_uri = Some(proof_key.as_presigned_url(&self.hostname));
                    })
                    .await;

                tracing::debug!(?self.request.id, "Proof fullfilled");
            }
            Err(err) => {
                tracing::error!("Failed to prove {}: {err}", self.request.id);

                let body = FailFulfillmentRequestBody {
                    nonce: nonce.nonce,
                    request_id: self.request.id.to_vec(),
                    error: None,
                };

                // Set the proof as unfulfillable on the prover network
                let fail_fulfill_resp = network_client
                    .fail_fulfillment(FailFulfillmentRequest {
                        format: MessageFormat::Binary.into(),
                        signature: body.sign(&self.fulfiller_signer).await?,
                        body: Some(body),
                    })
                    .await?
                    .into_inner();

                tracing::debug!(?self.request.id, "Proof marked as unfulfillable");

                self.db
                    .update_request(self.request.id, |r| {
                        r.fulfillment_status = FulfillmentStatus::Unfulfillable;
                        r.fulfill_tx_hash = Some(fail_fulfill_resp.tx_hash.clone());
                    })
                    .await;
            }
        }

        Ok(())
    }
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
