#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
mod omniflix;
pub mod state;

// The OmniflixApp + omniflix-std stargate harness needs prost 0.13 and
// the cw-multi-test 2.x AppBuilder semantics; omniflix-std 1.1.0-beta
// pulled in prost 0.13 transitively but the workspace still pins
// prost 0.12 for cosm-orc compatibility, leaving the test surface in
// dual-version-prost limbo. Gated off until we either bump the
// workspace prost pin or split the test harness out behind its own
// feature. The contract itself compiles + ships fine.
#[cfg(all(test, any()))]
mod testing;

pub use crate::error::ContractError;
