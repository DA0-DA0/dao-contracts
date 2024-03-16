#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod proposal;
pub mod query;

#[cfg(test)]
mod testing;

pub mod state;
pub mod v1_state;

pub use crate::error::ContractError;
