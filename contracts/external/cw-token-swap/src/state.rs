use cw_storage_plus::Item;

use crate::types::CheckedCounterparty;

pub const COUNTERPARTY_ONE: Item<CheckedCounterparty> = Item::new("counterparty_one");
pub const COUNTERPARTY_TWO: Item<CheckedCounterparty> = Item::new("counterparty_two");
