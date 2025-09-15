use clap::Parser;
use rustls::crypto::aws_lc_rs;
use tokio::signal;
use tracing::info;

use crate::{cli::Args, fulfiller::run};

mod cli;
mod fulfiller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    sp1_sdk::utils::setup_logger();
    aws_lc_rs::default_provider().install_default().unwrap();

    let args = Args::parse();

    info!("Starting fulfiller...");

    run(
        args.network_rpc_url,
        args.private_server_rpc_url,
        args.network_private_key,
        args.programs_s3_region,
        args.worker_count,
    )
    .await?;

    signal::ctrl_c().await?;

    Ok(())
}
