#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod state;
pub mod types;
pub mod utils;

// dao-migrator test surface targeted v1 DAOs; it's gated off with the rest of
// the v1 migration code. Re-enable once the v1 -> v2.9+ shim lands.
#[cfg(all(test, any()))]
mod testing;

pub use crate::error::ContractError;
