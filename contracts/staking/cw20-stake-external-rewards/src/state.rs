use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256};
use cw20::Denom;

use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub staking_contract: Addr,
    pub reward_token: Denom,
}

// `"config"` key stores v1 configuration.
pub const CONFIG: Item<Config> = Item::new("config_v2");

#[cw_serde]
pub struct RewardConfig {
    pub period_finish: u64,
    pub reward_rate: Uint128,
    pub reward_duration: u64,
}
pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");

pub const REWARD_PER_TOKEN: Item<Uint256> = Item::new("reward_per_token");

pub const LAST_UPDATE_BLOCK: Item<u64> = Item::new("last_update_block");

pub const PENDING_REWARDS: Map<Addr, Uint128> = Map::new("pending_rewards");

pub const USER_REWARD_PER_TOKEN: Map<Addr, Uint256> = Map::new("user_reward_per_token");
