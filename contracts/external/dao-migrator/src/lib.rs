#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
pub mod state;
mod error;
pub mod msg;

#[cfg(test)]
mod testing;

pub use crate::error::ContractError;
