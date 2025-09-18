## SP1 Private Proving

### Overview

This repository address the use case when customers wants fast proofs generation (on GPUs), but can't use a prover network because the proof inputs need to be private.

The repository is organized in 2 binaries, both designed to run inside a TEE:

* `server`: Acts as a proxy to the Succinct Prover Network, accepting proof requests from the SP1 SDK.
* `fulfiller`: Handle proving tasks and report proof requests status change to the Prover Network.

The TEE infra rely on [Phala cloud] and [dstack], and proving is done with [SP1].

To enable private proving with the SP1 SDK, you just need to call the [`private()`] fn on [`NetworkProverBuilder`]:

```rust
let client = ProverClient::builder()
    .network()
    .private()
    .build();
```

This will update the network prover client to send proof requests to a TEE application (via the sp1-lumiere.xyz domain) instead of sending them to the [Prover Network], allowing the proof inputs to remain private.

### Inputs Privacy Verification

In order to ensure the communications to the TEE enclaves are secure, the sp1-lumiere.xyz domain certificates must be managed by the TEE application itself. This is achieved by the Phala [Zero Trust TLS] protocol.

Phala also provides mechanisms for anyone to verify and attest that the domain is managed by the TEE application. The process is descibed in the [Domain Attestation and Verification] section in the Phala Cloud documentation, and [dstack-verifier], a tool to automate certificate verification is provided.

[`private()`]: https://docs.rs/sp1-sdk/latest/sp1_sdk/network/builder/struct.NetworkProverBuilder.html#method.private
[`NetworkProverBuilder`]: https://docs.rs/sp1-sdk/latest/sp1_sdk/network/builder/struct.NetworkProverBuilder.html
[Prover Network]: https://docs.succinct.xyz/docs/sp1/prover-network/quickstart
[SP1]: https://docs.succinct.xyz/docs/sp1/introduction
[Phala cloud]: https://docs.phala.com/phala-cloud/what-is/what-is-phala-cloud
[dstack]: https://github.com/Dstack-TEE/dstack
[Zero Trust TLS]: https://docs.phala.com/dstack/design-documents/whitepaper#zero-trust-tls-protocol
[Domain Attestation and Verification]: https://docs.phala.com/phala-cloud/networking/setting-up-custom-domain#domain-attestation-and-verification
[dstack-verifier]: https://github.com/Phala-Network/dstack-verifier
