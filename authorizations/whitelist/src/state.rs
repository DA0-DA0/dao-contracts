use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const DAO: Item<Addr> = Item::new("dao");
pub const AUTHORIZED: Map<String, cosmwasm_std::Empty> = Map::new("authorized");
