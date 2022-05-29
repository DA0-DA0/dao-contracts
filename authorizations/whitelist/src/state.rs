use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const DAO: Item<Addr> = Item::new("dao");
pub const AUTHORIZED: Map<Addr, cosmwasm_std::Empty> = Map::new("authorized");
pub const AUTHORIZED_GROUPS: Map<String, cosmwasm_std::Empty> = Map::new("authorized_groups");
