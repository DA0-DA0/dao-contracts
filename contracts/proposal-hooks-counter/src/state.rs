use cw_storage_plus::Item;

pub const VOTE_COUNTER: Item<u64> = Item::new("vote_counter");
pub const PROPOSAL_COUNTER: Item<u64> = Item::new("proposal_counter");
pub const STATUS_CHANGED_COUNTER: Item<u64> = Item::new("stauts_changed_counter");
