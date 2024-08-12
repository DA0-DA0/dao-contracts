#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

// Integrationg tests using an actual chain binary, requires
// the "test-tube" feature to be enabled
// cargo test --features test-tube
#[cfg(test)]
#[cfg(feature = "test-tube")]
mod integration_tests;

pub use crate::error::ContractError;
