#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod contract;
mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

pub use crate::error::RolesContractError as ContractError;

// So consumers don't need dependencies to interact with this contract.
pub use cw721_base::MinterResponse;
pub use cw_ownable::{Action, Ownership};
pub use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt, QueryExt};
