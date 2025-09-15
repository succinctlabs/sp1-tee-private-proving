use prost::Message;
use sp1_sdk::NetworkSigner;

pub trait Signable: Message {
    async fn sign(&self, signer: &NetworkSigner) -> anyhow::Result<Vec<u8>>;
}

impl<T: Message> Signable for T {
    async fn sign(&self, signer: &NetworkSigner) -> anyhow::Result<Vec<u8>> {
        let signature = signer.sign_message(self.encode_to_vec().as_slice()).await?;
        Ok(signature.as_bytes().to_vec())
    }
}
