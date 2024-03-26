pub mod abc;
pub(crate) mod commands;
pub mod contract;
pub mod curves;
mod error;
pub mod msg;
mod queries;
pub mod state;

// Integrationg tests using an actual chain binary, requires
// the "test-tube" feature to be enabled
// cargo test --features test-tube
#[cfg(test)]
#[cfg(feature = "test-tube")]
mod test_tube;

#[cfg(test)]
mod testing;

pub use crate::error::ContractError;
