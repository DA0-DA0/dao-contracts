#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

mod cell;
pub mod config;
pub mod contract;
mod error;
mod m;
pub mod msg;
pub mod proposal;
pub mod state;
pub mod tally;
#[cfg(test)]
mod tests;
pub mod vote;

pub use crate::error::ContractError;
