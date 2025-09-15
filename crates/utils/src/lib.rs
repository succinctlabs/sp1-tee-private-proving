use std::time::Duration;

use sp1_sdk::network::proto::network::prover_network_client::ProverNetworkClient;
use sp1_tee_private_types::prover_network_client::ProverNetworkClient as PrivateNetworkClient;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Error};

mod artifacts;
pub use artifacts::{generate_id, presigned_url};

mod signable;
pub use signable::Signable;

/// Configures the endpoint for the gRPC client.
///
/// Sets reasonable settings to handle timeouts and keep-alive.
pub fn configure_endpoint(addr: &str) -> Result<Endpoint, Error> {
    let mut endpoint = Endpoint::new(addr.to_string())?
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .keep_alive_while_idle(true)
        .http2_keep_alive_interval(Duration::from_secs(15))
        .keep_alive_timeout(Duration::from_secs(15))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .tcp_nodelay(true);

    // Configure TLS if using HTTPS.
    if addr.starts_with("https://") {
        let tls_config = ClientTlsConfig::new().with_enabled_roots();
        endpoint = endpoint.tls_config(tls_config)?;
    }

    Ok(endpoint)
}

pub async fn prover_network_client(rpc_url: &str) -> Result<ProverNetworkClient<Channel>, Error> {
    let channel = configure_endpoint(rpc_url)?.connect().await?;
    Ok(ProverNetworkClient::new(channel))
}

pub async fn private_network_client(rpc_url: &str) -> Result<PrivateNetworkClient<Channel>, Error> {
    let channel = configure_endpoint(rpc_url)?.connect().await?;
    Ok(PrivateNetworkClient::new(channel))
}
