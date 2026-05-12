#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod proposal;
pub mod query;

#[cfg(test)]
mod testing;

pub mod state;
// v1_state holds v1 -> v2 type-conversion helpers. The v1 migration path
// itself is stubbed out in this binary (see `migrate`) because cosmwasm-std
// 1.x and 2.x produce distinct `Storage` / `Addr` / `Decimal` / `Uint128` /
// `Timestamp` trait identities. Gated behind a never-true cfg so the file
// can stay in the tree as a reference for the eventual v1 -> v2.9+ shim.
#[cfg(any())]
pub mod v1_state;

pub use crate::error::ContractError;
