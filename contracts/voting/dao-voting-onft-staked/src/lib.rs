#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
mod omniflix;
pub mod state;

#[cfg(test)]
mod testing;

pub use crate::error::ContractError;
