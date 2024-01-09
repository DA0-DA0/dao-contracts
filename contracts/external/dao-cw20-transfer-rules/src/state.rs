use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

/// DAO address
pub const DAO: Item<Addr> = Item::new("dao");

/// The DAO voting module address
pub const DAO_VOTING_MODULE: Item<Addr> = Item::new("dao_voting_module");

/// Addresses allowed to transfer tokens even if not on the allowlist
pub const ALLOWLIST: Map<&Addr, ()> = Map::new("allowlist");
