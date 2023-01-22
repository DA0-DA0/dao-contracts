#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

// so that consumers don't need a cw_ownable dependency to consume this contract's queries.
pub use cw_ownable::Ownership;

pub use crate::error::ContractError;
