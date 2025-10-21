use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    routing::{get, put},
};
use clap::Parser;
use rustls::crypto::aws_lc_rs;
use serde::{Deserialize, Serialize};
use sp1_sdk::network::proto::artifact::artifact_store_server::ArtifactStoreServer;
use sp1_tee_private_types::prover_network_server::ProverNetworkServer;
use tonic::service::Routes;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::{
    artifact_routes::{download_artifact, upload_artifact},
    cli::Args,
    db::InMemoryDb,
    server::{DefaultArtifactStoreServer, DefaultPrivateProverServer},
};

mod artifact_routes;
mod cli;
mod db;
mod server;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    sp1_sdk::utils::setup_logger();
    aws_lc_rs::default_provider().install_default().unwrap();

    let args = Args::parse();

    info!("Starting server on port {}...", args.server_port);

    let db = Arc::new(InMemoryDb::new());

    let mut routes_builder = Routes::builder();

    routes_builder.add_service(ProverNetworkServer::new(DefaultPrivateProverServer::new(
        args.hostname.clone(),
        args.network_rpc_url.clone(),
        args.fulfiller_private_key.clone(),
        args.artifacts_port,
        db.clone(),
    )));

    routes_builder.add_service(ArtifactStoreServer::new(
        DefaultArtifactStoreServer::new(
            args.hostname.clone(),
            args.network_rpc_url.clone(),
            db.clone(),
        )
        .await,
    ));

    let grpc_routes = routes_builder.routes().into_axum_router();

    let server = Router::new()
        .route("/artifacts/stdin/:id", put(upload_artifact))
        .route("/health", get(health))
        .with_state(db.clone())
        .merge(grpc_routes)
        .layer(CorsLayer::permissive());

    let download_artifacts = Router::new()
        .route("/artifacts/stdin/:id", get(download_artifact))
        .with_state(db);

    let server_listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.server_port))
        .await
        .unwrap();

    let artifacts_listener =
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.artifacts_port))
            .await
            .unwrap();

    let (server_result, artifacts_result) = tokio::join!(
        axum::serve(server_listener, server),
        axum::serve(artifacts_listener, download_artifacts)
    );

    server_result.unwrap();
    artifacts_result.unwrap();
}

async fn health(State(db): State<Arc<InMemoryDb>>) -> Json<HealthResponse> {
    let response = HealthResponse {
        queued_proof_request_count: db.queued_proof_request_count().await,
    };

    Json(response)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    queued_proof_request_count: usize,
}
