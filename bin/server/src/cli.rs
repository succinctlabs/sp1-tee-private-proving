use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The port to listen on.
    #[clap(short, long, default_value = "8080")]
    pub port: u16,
}
