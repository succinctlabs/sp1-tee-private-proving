use sp1_sdk::{Prover, ProverClient, SP1Stdin, network::FulfillmentStrategy};

const FIBONACCI_ELF: &[u8] = include_bytes!("../fixtures/fibonacci.elf");

#[tokio::test(flavor = "multi_thread")]
async fn test_prove() {
    sp1_sdk::utils::setup_logger();

    let client = ProverClient::builder()
        .network()
        .private()
        .private_key("0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d") // 0x70997970C51812dc3A010C7d01b50e0d17dc79C8
        .build();

    let (pk, vk) = client.setup(FIBONACCI_ELF);
    let mut stdin = SP1Stdin::new();

    stdin.write(&10);

    let proof = client
        .prove(&pk, &stdin)
        .groth16()
        .strategy(FulfillmentStrategy::Reserved)
        .skip_simulation(true)
        .run()
        .unwrap();

    client.verify(&proof, &vk).unwrap();
}
