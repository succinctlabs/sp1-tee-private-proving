## SP1 Private Proving

### Overview

This repository contains 2 binaries designed to bring proof generation to a TEE enclave running GPUs, allowing the proofs input data to no be sent to the Succinct [Prover Network], thus remaining private:

* `server`: Acts as a proxy to the Succinct Prover Network, accepting proof requests from the SP1 SDK.
* `fulfiller`: Handle proving tasks and report proof requests status change to the Prover Network.

To enable private proving with the SP1 SDK, you just need to call the [`private()`] fn on [`NetworkProverBuilder`]:

```rust
let client = ProverClient::builder()
    .network()
    .private()
    .build();
```

This will update the network prover client to send proof requests to the TEE enclave instead of sending them to the [Prover Network], allowing the proof inputs to remain private.

### Inputs Privacy Verification

The sp1-lumiere.xyz domain certificates are managed by the TEE enclave itself thanks to the Phala [Zero Trust TLS] protocol. This ensures that the proofs input data sent to the TEE remains private.

Phala also provides mechanisms to verify and attest that the domain is managed by the TEE enclave. The process is descibed in the [Domain Attestation and Verification] section in the Phala Cloud documentation.


[`private()`]: https://docs.rs/sp1-sdk/latest/sp1_sdk/network/builder/struct.NetworkProverBuilder.html#method.private
[`NetworkProverBuilder`]: https://docs.rs/sp1-sdk/latest/sp1_sdk/network/builder/struct.NetworkProverBuilder.html
[Prover Network]: https://docs.succinct.xyz/docs/sp1/prover-network/quickstart
[Zero Trust TLS]: https://docs.phala.com/dstack/design-documents/whitepaper#zero-trust-tls-protocol
[Domain Attestation and Verification]: https://docs.phala.com/phala-cloud/networking/setting-up-custom-domain#domain-attestation-and-verification