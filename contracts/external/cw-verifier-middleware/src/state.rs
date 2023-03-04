use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

/// Nonce for each public key
pub const NONCES: Map<&str, Uint128> = Map::new("pk_to_nonce");

/// Contract address for which this middleware is used.
/// We require the contract address as part of the
/// payload to prevent replay attacks across contracts (a nonce may be used multiple times if there is no other
/// way to determine that it has already be used).
pub const CONTRACT_ADDRESS: Item<String> = Item::new("contract_address");
