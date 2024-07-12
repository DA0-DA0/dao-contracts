use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// The account allowed to execute the contract. If None, anyone is allowed.
pub const ADMIN: Item<Option<Addr>> = Item::new("admin");

/// The expected instantiate2 address to validate in the reply.
pub const EXPECT: Item<Addr> = Item::new("expect");
