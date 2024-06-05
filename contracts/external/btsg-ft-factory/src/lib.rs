#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod bitsong;
pub mod contract;
mod error;
pub mod msg;

mod shim;

pub use crate::error::ContractError;
