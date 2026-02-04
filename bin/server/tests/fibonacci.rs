use sp1_sdk::{Prover, ProverClient, SP1Stdin, network::FulfillmentStrategy};

const FIBONACCI_ELF: &[u8] = include_bytes!("../fixtures/fibonacci.elf");

#[tokio::test(flavor = "multi_thread")]
async fn test_prove() {
    dotenv::dotenv().ok();
    sp1_sdk::utils::setup_logger();

    let client = ProverClient::builder().network().private().build();

    let (pk, vk) = client.setup(FIBONACCI_ELF);
    let mut stdin = SP1Stdin::new();

    stdin.write(&10);

    let (_, report) = client.execute(FIBONACCI_ELF, &stdin).run().unwrap();

    println!("Cycles: {}", report.total_instruction_count());
    println!("Gas used: {}", report.gas.unwrap());

    let proof = client
        .prove(&pk, &stdin)
        .groth16()
        .strategy(FulfillmentStrategy::Reserved)
        .skip_simulation(true)
        .run()
        .unwrap();

    client.verify(&proof, &vk).unwrap();
}
