pub mod contract;
mod error;
pub mod hooks;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;

#[cfg(test)]
mod tests_cw20_base;

#[cfg(test)]
mod tests;
