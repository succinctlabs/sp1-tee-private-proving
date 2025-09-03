use std::{io::Read, sync::Arc};

use anyhow::anyhow;
use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use sp1_sdk::{SP1Stdin, network::proto::artifact::ArtifactType};
use tokio::{sync::oneshot, task::spawn_blocking};
use tokio_util::io::{StreamReader, SyncIoBridge};

use crate::{
    db::{Db, InMemoryDb},
    types::Key,
};

pub async fn upload_artifact(
    Path((ty, id)): Path<(String, String)>,
    State(db): State<Arc<InMemoryDb>>,
    body: Body,
) -> StatusCode {
    tracing::info!("start upload artifact");
    let ty = ArtifactType::from_str_name(&ty.to_uppercase()).unwrap_or_default();
    let stream = body.into_data_stream().map_err(std::io::Error::other);
    let async_reader = StreamReader::new(stream);
    let sync_reader = SyncIoBridge::new(async_reader);

    if !db.consume_artifact_request(Key::new(&ty, &id)).await {
        return StatusCode::UNAUTHORIZED;
    }

    match ty {
        ArtifactType::Stdin => match deserialize::<_, SP1Stdin>(sync_reader).await {
            Ok(stdin) => {
                db.insert_artifact(Key::new(&ty, &id), stdin.into()).await;
                StatusCode::OK
            }
            Err(err) => {
                tracing::error!("Failed to deserialize sdtin artifact: {err}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        },
        _ => StatusCode::NOT_FOUND,
    }
}

pub async fn download_artifact(
    Path((ty, id)): Path<(String, String)>,
    State(db): State<Arc<InMemoryDb>>,
) -> Result<Vec<u8>, StatusCode> {
    let ty = ArtifactType::from_str_name(&ty.to_uppercase()).unwrap_or_default();

    if matches!(ty, ArtifactType::Proof) {
        match db.get_proof(Key::new(&ty, &id)).await {
            Some(proof) => {
                let proof_bytes = bincode::serialize(proof.as_ref()).map_err(|err| {
                    tracing::error!("Failed to serialize proof: {err}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
                Ok(proof_bytes)
            }
            None => Err(StatusCode::NOT_FOUND),
        }
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

async fn deserialize<R: Read + Send + 'static, T: DeserializeOwned + Send + 'static>(
    mut reader: R,
) -> Result<T, anyhow::Error> {
    let (tx, rx) = oneshot::channel();

    spawn_blocking(move || {
        let mut buf = vec![];
        if let Err(err) = reader.read_to_end(&mut buf).map_err(|err| anyhow!("{err}")) {
            let _ = tx.send(Err(err));
            return;
        }

        tracing::info!("start deserialize");
        let artifact = bincode::deserialize::<T>(&buf).map_err(|err| anyhow!("{err}"));

        let _ = tx.send(artifact);
    });

    rx.await.unwrap()
}
