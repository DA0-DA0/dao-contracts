#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod state;
pub mod vesting;

pub use crate::error::ContractError;

// so that consumers don't need a cw_ownable dependency to consume this contract's queries.
pub use cw_ownable::Ownership;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod vesting_tests;
