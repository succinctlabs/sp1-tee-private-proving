mod key;
pub use key::Key;

mod request;
pub use request::{PendingRequest, ProofRequest};

include!(concat!(env!("OUT_DIR"), "/network.ProverNetwork.rs"));
