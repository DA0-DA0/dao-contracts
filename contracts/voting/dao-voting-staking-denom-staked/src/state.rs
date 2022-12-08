use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const STAKING_MODULE: Item<Addr> = Item::new("staking_module");
pub const DAO: Item<Addr> = Item::new("dao");
