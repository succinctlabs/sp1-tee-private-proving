use sp1_sdk::{Prover, ProverClient, SP1Stdin};

const FIBONACCI_ELF: &[u8] = include_bytes!("../fixtures/fibonacci.elf");

#[tokio::test(flavor = "multi_thread")]
async fn test_prove() {
    sp1_sdk::utils::setup_logger();

    let client = ProverClient::builder()
        .private()
        .private_key("0xbcdf20249abf0ed6d944c0288fad489e33f66b3960d9e6229c1cd214ed3bbe31")
        .rpc_url("https://leruaa-private.work:18364/")
        //.rpc_url("http://localhost:8888/")
        .build();

    let (pk, vk) = client.setup(FIBONACCI_ELF);
    let mut stdin = SP1Stdin::new();

    stdin.write(&10);

    let proof = client
        .prove(&pk, &stdin)
        .skip_simulation(true)
        .run()
        .unwrap();

    client.verify(&proof, &vk).unwrap();
}
