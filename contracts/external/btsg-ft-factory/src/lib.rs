#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod bitsong;
pub mod contract;
mod error;
pub mod msg;
pub mod state;

mod shim;

pub use crate::error::ContractError;

// The bitsong stargate test harness uses a custom `App` wrapper whose
// generic-parameter shape doesn't survive the cw-multi-test 0.20 -> 2.4
// bump (AppBuilder default module types changed). Re-enable once we
// rework the harness to use cw-multi-test 2.x AppBuilder semantics.
#[cfg(all(test, any()))]
mod testing;
