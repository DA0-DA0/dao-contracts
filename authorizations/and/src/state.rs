use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const PARENT: Item<Addr> = Item::new("dao");
pub const CHILDREN: Map<Addr, cosmwasm_std::Empty> = Map::new("children");
