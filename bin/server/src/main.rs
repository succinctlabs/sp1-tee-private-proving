use clap::Parser;
use sp1_sdk::private::proto::private_prover_server::PrivateProverServer;
use tonic::transport::Server;
use tracing::info;

use crate::{cli::Args, db::InMemoryDb, server::DefaultPrivateProverServer};

mod cli;
mod db;
mod fulfiller;
mod server;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    sp1_sdk::utils::setup_logger();

    let db = InMemoryDb::new();
    let args = Args::parse();

    info!("Starting server...");
    Server::builder()
        .add_service(PrivateProverServer::new(DefaultPrivateProverServer::new(
            db,
        )))
        .serve(format!("[::1]:{}", args.port).parse().unwrap())
        .await
        .unwrap();
}
