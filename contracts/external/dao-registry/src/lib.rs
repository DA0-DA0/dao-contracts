#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
pub mod error;
pub mod msg;
pub mod registration;
pub mod state;

pub use crate::error::ContractError;

// Consumers don't need a cw_ownable dependency to use this contract's queries.
pub use cw_denom::{CheckedDenom, UncheckedDenom};
pub use cw_ownable::Ownership;

#[cfg(test)]
mod tests;
