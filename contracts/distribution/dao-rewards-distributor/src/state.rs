use cosmwasm_std::{Addr, Uint128, Uint256};
use cw_storage_plus::{Item, Map};
use std::collections::HashMap;

use crate::msg::RewardConfig;

pub const REWARDS_PER_TOKEN: Item<HashMap<String, Uint256>> = Item::new("rewards_per_token");

/// A map of user addresses to their pending rewards.
pub const PENDING_REWARDS: Map<Addr, HashMap<String, Uint128>> = Map::new("pending_rewards");

/// A map of user addresses to their rewards per token. In other words, it is the
/// reward per share of voting power that the user has.
pub const USER_REWARD_PER_TOKEN: Map<Addr, HashMap<String, Uint256>> =
    Map::new("user_reward_per_token");

pub const FUNDED_DENOM_AMOUNTS: Map<String, Uint128> = Map::new("funded_denom_amounts");

// registered hooks mapping to denoms they are registered for
pub const REGISTERED_HOOKS: Map<Addr, Vec<String>> = Map::new("registered_hook_callers");

/// voting power contract address that is used to calculate base rewards
pub const MAIN_VP_CONTRACT: Item<(Addr, Vec<String>)> = Item::new("main_vp_contract");

/// maps denom str to its reward configuration
pub const REWARD_DENOM_CONFIGS: Map<String, RewardConfig> = Map::new("rdc");
