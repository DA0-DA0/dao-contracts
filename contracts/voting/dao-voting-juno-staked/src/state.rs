use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// The address of the DAO that instantiated this voting module.
pub const DAO: Item<Addr> = Item::new("dao");
