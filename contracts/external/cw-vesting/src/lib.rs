#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod vesting;

pub use crate::error::ContractError;

// so consumers don't need a cw_ownable dependency to use this contract's queries.
pub use cw_denom::{CheckedDenom, UncheckedDenom};
pub use cw_ownable::Ownership;

// so consumers don't need a cw_stake_tracker dependency to use this contract's queries.
pub use cw_stake_tracker::StakeTrackerQuery;

#[cfg(test)]
mod suite_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vesting_tests;
