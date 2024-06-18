#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod bitsong;
pub mod contract;
mod error;
pub mod msg;
pub mod state;

mod shim;

pub use crate::error::ContractError;

#[cfg(test)]
mod testing;
