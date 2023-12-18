pub mod msg;

pub mod cosmwasm;
pub use osmosis_std::types::osmosis::tokenfactory::v1beta1 as osmosis;

// helpers for both osmosis types (osmosis_std crate) and cosmwasm types. it
// needs to be named `shim` because osmosis_std assumes it exists.
mod shim;
