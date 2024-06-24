use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// The address of the DAO this voting contract is connected to.
pub const DAO: Item<Addr> = Item::new("dao");
