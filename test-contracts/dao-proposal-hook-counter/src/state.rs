use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub should_error: bool,
}
pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNTER: Item<u64> = Item::new("proposal_counter");
pub const STAKE_COUNTER: Item<Uint128> = Item::new("stake_counter");
pub const STATUS_CHANGED_COUNTER: Item<u64> = Item::new("stauts_changed_counter");
pub const VOTE_COUNTER: Item<u64> = Item::new("vote_counter");
