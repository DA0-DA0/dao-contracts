#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod action;
pub mod contract;
mod error;
pub mod helpers;
pub mod msg;
mod protobuf;
pub mod role;
pub mod shim;
pub mod state;

#[cfg(test)]
mod testing;

pub use crate::error::ContractError;
