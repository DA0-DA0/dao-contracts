use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ROOT: Item<Addr> = Item::new("root");
pub const DAO: Item<Addr> = Item::new("dao");
