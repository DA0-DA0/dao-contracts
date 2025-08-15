#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod action;
pub mod contract;
mod error;
pub mod helpers;
pub mod msg;
pub mod role;
pub mod state;

#[cfg(test)]
pub mod shim;
#[cfg(test)]
mod testing;

pub use crate::error::ContractError;
