use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const GROUPS: Map<&str, Vec<Addr>> = Map::new("groups");
