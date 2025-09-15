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

    /// The port for the server.
    #[clap(short, long, default_value = "8080")]
    pub server_port: u16,

    /// The port for the artifacts download.
    #[clap(short, long, default_value = "8081")]
    pub artifacts_port: u16,
}
