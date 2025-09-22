use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const TEE_DOMAIN: &str = "https://tee.sp1-lumiere.xyz";
const PHALA_CLOUD_API: &str = "https://cloud-api.phala.network/api/v1";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http_client = reqwest::Client::new();

    let quote = http_client
        .get(format!("{TEE_DOMAIN}/evidences/quote.json"))
        .send()
        .await?
        .json::<QuoteResponse>()
        .await?;

    let attestation = http_client
        .post(format!("{PHALA_CLOUD_API}/attestations/verify"))
        .json(&json!({ "hex": format!("0x{}", quote.quote) }))
        .send()
        .await?
        .json::<AttestationResponse>()
        .await?;

    println!("rtmr3: {}", attestation.quote.body.rtmr3);

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuoteResponse {
    quote: String,
    event_log: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttestationResponse {
    success: bool,
    quote: Quote,
    checksum: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Quote {
    header: QuoteHeader,
    body: QuoteBody,
    cert_data: String,
    verified: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuoteHeader {
    version: u8,
    ak_type: String,
    tee_type: String,
    qe_vendor: String,
    user_data: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuoteBody {
    tee_tcb_svn: String,
    mrseam: String,
    mrtd: String,
    rtmr0: String,
    rtmr1: String,
    rtmr2: String,
    rtmr3: String,
    reportdata: String,
}
