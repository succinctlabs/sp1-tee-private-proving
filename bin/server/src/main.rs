use std::sync::Arc;

use axum::{
    Router,
    routing::{get, put},
};
use clap::Parser;
use sp1_sdk::network::proto::artifact::artifact_store_server::ArtifactStoreServer;
use sp1_tee_private_types::prover_network_server::ProverNetworkServer;
use tonic::service::Routes;
use tracing::info;

use crate::{
    artifact_routes::{download_artifact, upload_artifact},
    artifact_store::DefaultArtifactStoreServer,
    cli::Args,
    db::InMemoryDb,
    server::DefaultPrivateProverServer,
};

mod artifact_routes;
mod artifact_store;
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

    let mut routes_builder = Routes::builder();

    routes_builder.add_service(ProverNetworkServer::new(DefaultPrivateProverServer::new(
        args.hostname.clone(),
        db.clone(),
        args.worker_count,
    )));

    routes_builder.add_service(ArtifactStoreServer::new(
        DefaultArtifactStoreServer::new(args.hostname.clone(), db.clone()).await,
    ));

    let grpc_routes = routes_builder.routes().into_axum_router();

    let app = Router::new()
        .route("/artifacts/:type/:key", put(upload_artifact))
        .route("/artifacts/:type/:key", get(download_artifact))
        .with_state(db)
        .merge(grpc_routes);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
