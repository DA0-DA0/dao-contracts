use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Addr> = Item::new("admin");
/// Maps priorities to addresses.
pub const ITEMS: Map<u32, Addr> = Map::new("items");
