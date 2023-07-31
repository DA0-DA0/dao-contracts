use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const GROUP_CONTRACT: Item<Addr> = Item::new("group_contract");
pub const DAO: Item<Addr> = Item::new("dao_address");
