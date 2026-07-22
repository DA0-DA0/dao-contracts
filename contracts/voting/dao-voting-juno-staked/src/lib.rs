#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod bindings;
pub mod contract;
mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

pub use crate::error::ContractError;
pub use contract::{execute, instantiate, migrate, query};
