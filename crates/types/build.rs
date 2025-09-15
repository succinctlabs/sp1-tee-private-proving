extern crate prost_build;
extern crate tonic_build;

#[allow(deprecated)]
fn main() {
    let network_service = tonic_build::manual::Service::builder()
        .name("ProverNetwork")
        .package("network")
        .method(
            tonic_build::manual::Method::builder()
                .name("create_program")
                .route_name("CreateProgram")
                .input_type("sp1_sdk::network::proto::types::CreateProgramRequest")
                .output_type("sp1_sdk::network::proto::types::CreateProgramResponse")
                .codec_path("tonic::codec::ProstCodec")
                .build(),
        )
        .method(
            tonic_build::manual::Method::builder()
                .name("get_program")
                .route_name("GetProgram")
                .input_type("sp1_sdk::network::proto::types::GetProgramRequest")
                .output_type("sp1_sdk::network::proto::types::GetProgramResponse")
                .codec_path("tonic::codec::ProstCodec")
                .build(),
        )
        .method(
            tonic_build::manual::Method::builder()
                .name("get_nonce")
                .route_name("GetNonce")
                .input_type("sp1_sdk::network::proto::types::GetNonceRequest")
                .output_type("sp1_sdk::network::proto::types::GetNonceResponse")
                .codec_path("tonic::codec::ProstCodec")
                .build(),
        )
        .method(
            tonic_build::manual::Method::builder()
                .name("request_proof")
                .route_name("RequestProof")
                .input_type("sp1_sdk::network::proto::types::RequestProofRequest")
                .output_type("sp1_sdk::network::proto::types::RequestProofResponse")
                .codec_path("tonic::codec::ProstCodec")
                .build(),
        )
        .method(
            tonic_build::manual::Method::builder()
                .name("take_next_proof_request")
                .route_name("TakeNextProofRequest")
                .input_type("crate::Unit")
                .output_type("sp1_sdk::network::proto::types::ProofRequest")
                .codec_path("tonic::codec::ProstCodec")
                .build(),
        )
        .method(
            tonic_build::manual::Method::builder()
                .name("get_proof_request_status")
                .route_name("GetProofRequestStatus")
                .input_type("sp1_sdk::network::proto::types::GetProofRequestStatusRequest")
                .output_type("sp1_sdk::network::proto::types::GetProofRequestStatusResponse")
                .codec_path("tonic::codec::ProstCodec")
                .build(),
        )
        .build();

    tonic_build::manual::Builder::new().compile(&[network_service]);
}
