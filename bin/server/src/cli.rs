use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The server public host name.
    #[clap(long, env)]
    pub hostname: String,

    /// The network private key.
    #[clap(long, env)]
    pub network_rpc_url: String,

    /// The netaork private key.
    #[clap(long, env)]
    pub network_private_key: String,

    /// The S3 region where programs are stored.
    #[clap(long, env)]
    pub programs_s3_region: String,

    /// The port to listen on.
    #[clap(short, long, default_value = "8080")]
    pub port: u16,

    #[clap(long, env, default_value = "1")]
    pub worker_count: usize,
}
