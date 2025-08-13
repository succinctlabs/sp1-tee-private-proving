use std::fmt::Display;

use crate::Key;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum ArtifactType {
    #[serde(rename = "unspecified")]
    UnspecifiedArtifactType = 0,
    /// A program artifact.
    #[serde(rename = "programs")]
    Program = 1,
    /// A stdin artifact.
    #[serde(rename = "stdins")]
    Stdin = 2,
    /// A proof artifact.
    #[serde(rename = "proofs")]
    Proof = 3,
}

impl ArtifactType {
    pub fn key(&self, id: &str) -> Key {
        Key::new(self, id)
    }
}

impl Display for ArtifactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactType::UnspecifiedArtifactType => write!(f, "artifacts"),
            ArtifactType::Program => write!(f, "programs"),
            ArtifactType::Stdin => write!(f, "stdins"),
            ArtifactType::Proof => write!(f, "proofs"),
        }
    }
}
