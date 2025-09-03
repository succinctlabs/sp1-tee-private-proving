use std::time::Duration;

use sp1_sdk::network::proto::artifact::ArtifactType;
use tonic::transport::{ClientTlsConfig, Endpoint, Error};

use crate::types::Key;

pub struct PresignedUrl {
    pub key: Key,
}

impl PresignedUrl {
    pub fn new(artifact_type: &ArtifactType) -> Self {
        Self {
            key: Key::generate(artifact_type),
        }
    }

    pub fn url(&self, hostname: &str) -> String {
        format!("{hostname}/artifacts/{}", self.key)
    }
}

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
