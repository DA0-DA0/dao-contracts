use std::collections::HashSet;

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");

// group --> addresses
pub const GROUPS: Map<&str, HashSet<Addr>> = Map::new("groups");

// address --> groups
pub const ADDRESSES: Map<&str, HashSet<String>> = Map::new("addresses");
