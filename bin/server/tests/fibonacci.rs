use sp1_sdk::{Prover, ProverClient, SP1Stdin};

const FIBONACCI_ELF: &[u8] = include_bytes!("../fixtures/fibonacci.elf");

#[tokio::test(flavor = "multi_thread")]
async fn test_prove() {
    sp1_sdk::utils::setup_logger();

    let client = ProverClient::builder()
        .private()
        .private_key("0xbcdf20249abf0ed6d944c0288fad489e33f66b3960d9e6229c1cd214ed3bbe31")
        .rpc_url("http://[::1]:8888/")
        .build();

    let (pk, _) = client.setup(FIBONACCI_ELF);
    let mut stdin = SP1Stdin::new();

    stdin.write(&10);

    client
        .prove(&pk, &stdin)
        .skip_simulation(true)
        .run()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn bench() {
    sp1_sdk::utils::setup_logger();
    tracing::info!("Start");
    let client = ProverClient::builder().mock().build();

    tracing::info!("Setup");
    let (pk, _) = client.setup(FIBONACCI_ELF);

    tracing::info!("Serialize");
    let bytes = bincode::serialize(&pk).unwrap();

    tracing::info!("Done: {}", bytes.len() / 1024 / 1024);
}
