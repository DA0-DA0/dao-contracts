use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256};
use cw20::Denom;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// The address of a DAO DAO voting power module contract.
    pub vp_contract: Addr,
    /// An optional contract that is allowed to call the StakeChangedHook in
    /// place of the voting power contract.
    pub hook_caller: Option<Addr>,
    /// The Denom in which rewards are paid out.
    pub reward_token: Denom,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct RewardConfig {
    pub period_finish: u64,
    pub reward_rate: Uint128,
    pub reward_duration: u64,
}
pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");

pub const REWARD_PER_TOKEN: Item<Uint256> = Item::new("reward_per_token");

pub const LAST_UPDATE_BLOCK: Item<u64> = Item::new("last_update_block");

/// A map of user addresses to their pending rewards.
pub const PENDING_REWARDS: Map<Addr, Uint128> = Map::new("pending_rewards");

/// A map of user addresses to their rewards per token. In other words, it is the
/// reward per share of voting power that the user has.
pub const USER_REWARD_PER_TOKEN: Map<Addr, Uint256> = Map::new("user_reward_per_token");
