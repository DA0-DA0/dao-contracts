use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::AddressItem;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const ITEMS: Item<BTreeSet<AddressItem>> = Item::new("items");
