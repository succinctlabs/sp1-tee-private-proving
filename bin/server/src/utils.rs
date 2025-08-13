use sp1_tee_private_types::{ArtifactType, Key};

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
