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

This will update the network prover client to send proof requests to a TEE application (via the tee.sp1-lumiere.xyz domain) instead of sending them to the [Prover Network], allowing the proof inputs to remain private.

### TLS Certificates Verification

In order to ensure the communications to the TEE enclaves are secure, the tee.sp1-lumiere.xyz domain certificates must be managed by the TEE application itself. This is achieved by the Phala [Zero Trust TLS] protocol.

Phala provides mechanisms for anyone to verify and attest that the domain is managed by the TEE application. The process is descibed in the [Domain Attestation] section in the Phala Cloud documentation, and [dstack-verifier], a tool to automate certificate verification is provided.

### Application Integrity Verificaation

Phala also provides mechanisms for anyone to verify that an application is running inside a genuine, secure TEE with the expected configuration and code, with Remote Attestation. You can learn more about it on the [Understanding Attestation] page in the Phala Cloud documentation.

To verify that the code running inside the TEE application at tee.sp1-lumiere.xyz correspond to the code published in this repo, follow these steps:


1. Clone the repository
   ```
   git clone https://github.com/succinctlabs/sp1-tee-private-proving.git
   cd sp1-tee-private-proving
   ```

2. Build the Docker images for the server and the fulfiller
   ```
   just build-docker-images
   ```

3. Display the Docker `server` and `fulfiller` images digests:
   ```
   just show-digests
   ```

4. Verify the digests correspond to the ones used in the `docker-compose.yml` file at the root path of this repo

5. Retrieve the known [RTMR3] of the TEE application from the Phala Cloud API:
   ```
   just show-digests
   ```

6. Compute the RTMR3 using the [RTMR3 Calculator], using the following values: 
   * The `docker-compose.yml` file is the one you verified at step 4
   * The appId is `9b78cf840e16a8274e00474cdac4afdabc5eeb93`
   * The InstanceId is `f15edb2cc265a06a7cb524d0022bb13277eb177c`

If both `RTMR3`s at step 5 and 6 are identical, you’ve proven that the `docker-compose.yml` file used to create the TEE Application is the same as the one in this repo contains the exact Docker images you built.


[`private()`]: https://docs.rs/sp1-sdk/latest/sp1_sdk/network/builder/struct.NetworkProverBuilder.html#method.private
[`NetworkProverBuilder`]: https://docs.rs/sp1-sdk/latest/sp1_sdk/network/builder/struct.NetworkProverBuilder.html
[Prover Network]: https://docs.succinct.xyz/docs/sp1/prover-network/quickstart
[SP1]: https://docs.succinct.xyz/docs/sp1/introduction
[Phala cloud]: https://docs.phala.com/phala-cloud/what-is/what-is-phala-cloud
[dstack]: https://github.com/Dstack-TEE/dstack
[Zero Trust TLS]: https://docs.phala.com/dstack/design-documents/whitepaper#zero-trust-tls-protocol
[Domain Attestation]: https://docs.phala.com/phala-cloud/networking/domain-attestation#custom-domains-zero-trust-verification
[Understanding Attestation]: https://docs.phala.com/phala-cloud/attestation/overview#introduction
[RTMR3]: https://docs.phala.com/phala-cloud/attestation/overview#rtmr3-event-chain%3A-how-application-components-are-measured
[RTMR3 Calculator]: https://rtmr3-calculator.vercel.app/
[dstack-verifier]: https://github.com/Phala-Network/dstack-verifier
