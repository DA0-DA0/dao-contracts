use cosmwasm_std::Uint128;
use cw_storage_plus::{Item};

pub const NONCE: Item<Uint128> = Item::new("nonce");