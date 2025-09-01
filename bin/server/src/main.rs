use std::sync::Arc;

use axum::{
    Router,
    routing::{get, put},
};
use clap::Parser;
use sp1_sdk::private::proto::private_prover_server::PrivateProverServer;
use tonic::service::Routes;
use tracing::info;

use crate::{
    artifact_routes::{download_artifact, upload_artifact},
    cli::Args,
    db::InMemoryDb,
    server::DefaultPrivateProverServer,
};

mod artifact_routes;
mod cli;
mod db;
mod fulfiller;
mod server;
mod utils;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    sp1_sdk::utils::setup_logger();

    let args = Args::parse();

    info!("Starting server on port {}...", args.port);

    let db = Arc::new(InMemoryDb::new());

    let grpc_service = PrivateProverServer::new(DefaultPrivateProverServer::new(
        args.hostname.clone(),
        db.clone(),
        args.worker_count,
    ));

    let app = Router::new()
        .route("/artifacts/:type/:key", put(upload_artifact))
        .route("/artifacts/:type/:key", get(download_artifact))
        .with_state(db)
        .merge(Routes::new(grpc_service).into_axum_router());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
