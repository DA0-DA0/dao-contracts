use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

// (user address, group addr) -> Empty
pub const GROUPS: Map<(&Addr, &Addr), Empty> = Map::new("groups");
