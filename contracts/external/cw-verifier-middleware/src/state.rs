use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

pub const NONCES: Map<&str, Uint128> = Map::new("pk_to_nonce");