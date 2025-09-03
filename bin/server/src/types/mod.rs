mod key;
pub use key::Key;

mod request;
pub use request::{PendingRequest, Request, UnfulfillableRequestReason};

include!(concat!(env!("OUT_DIR"), "/network.ProverNetwork.rs"));
