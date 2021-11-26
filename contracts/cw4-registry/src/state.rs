use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub static EMPTY: u16 = 0;

// (member address, group addr) -> Empty
pub const MEMBER_INDEX: Map<(&Addr, &Addr), u16> = Map::new("member_index");

// (Group address, user addr) -> Empty
pub const GROUP_INDEX: Map<(&Addr, &Addr), u16> = Map::new("group_index");
