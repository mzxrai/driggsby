pub mod commands;
pub mod contracts;
pub mod error;
mod import;
pub mod migrations;
pub mod setup;
pub mod state;

pub use contracts::envelope::{FailureEnvelope, SuccessEnvelope};
pub use error::{ClientError, ClientResult};

pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");
