use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const DAO_ADDRESS: Item<Addr> = Item::new("dao_address");
pub const TOKEN: Item<Addr> = Item::new("token");
