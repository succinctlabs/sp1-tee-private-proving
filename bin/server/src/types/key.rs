use std::fmt::Display;

use mti::prelude::{MagicTypeIdExt, V7};
use serde::Deserialize;
use sp1_sdk::network::proto::artifact::ArtifactType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Key(String);

impl Key {
    pub fn new(artifact_type: &ArtifactType, id: &str) -> Self {
        let artifact_name = artifact_type.as_str_name().to_lowercase();

        Self(format!("{artifact_name}/{id}"))
    }

    pub fn generate(artifact_type: &ArtifactType) -> Self {
        // Create a TypeID.
        let type_id = "artifact".create_type_id::<V7>();
        let id = type_id.to_string();

        Self::new(artifact_type, &id)
    }

    pub fn from_uri(uri: &str) -> Self {
        Self(uri.replace("artifacts://", ""))
    }

    pub fn as_uri(&self) -> String {
        format!("artifacts://{}", self.0)
    }

    pub fn as_presigned_url(&self, hostname: &str) -> String {
        format!("{hostname}/artifacts/{}", self.0)
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
