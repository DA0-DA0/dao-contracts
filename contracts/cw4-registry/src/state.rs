use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

// (user address, multisig addr) -> Empty
pub const INDEX: Map<(&Addr, &Addr), Empty> = Map::new("index");
