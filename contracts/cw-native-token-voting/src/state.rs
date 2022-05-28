use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const TOKEN_DENOM: Item<String> = Item::new("token_denom");
pub const DAO: Item<Addr> = Item::new("dao");
