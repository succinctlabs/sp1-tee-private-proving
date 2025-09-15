use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
};
use futures::TryStreamExt;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::db::{Db, InMemoryDb};

pub async fn upload_artifact(
    Path(id): Path<String>,
    State(db): State<Arc<InMemoryDb>>,
    body: Body,
) -> StatusCode {
    let stream = body.into_data_stream().map_err(std::io::Error::other);
    let mut async_reader = StreamReader::new(stream);

    tracing::debug!("Upload {id}");

    if !db.consume_artifact_request(id.clone()).await {
        return StatusCode::UNAUTHORIZED;
    }

    let mut buf = vec![];

    match async_reader.read_to_end(&mut buf).await {
        Ok(_) => {
            db.insert_stdin(id, buf).await;
            StatusCode::OK
        }
        Err(err) => {
            tracing::error!("Failed to read sdtin artifact: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn download_artifact(
    Path(id): Path<String>,
    State(db): State<Arc<InMemoryDb>>,
) -> Result<Vec<u8>, StatusCode> {
    match db.get_stdin(&id).await {
        Some(stdin) => Ok((*stdin).clone()),
        None => Err(StatusCode::NOT_FOUND),
    }
}
