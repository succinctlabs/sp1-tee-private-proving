use mti::prelude::{MagicTypeIdExt, V7};
use sp1_sdk::network::proto::artifact::ArtifactType;

pub fn generate_id() -> String {
    // Create a TypeID.
    let type_id = "artifact".create_type_id::<V7>();
    type_id.to_string()
}

pub fn presigned_url(hostname: &str, artifact_type: ArtifactType, id: &str) -> String {
    let artifact_name = artifact_type.as_str_name().to_lowercase();
    format!("{hostname}/artifacts/{artifact_name}/{id}")
}
