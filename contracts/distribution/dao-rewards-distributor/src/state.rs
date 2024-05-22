use cosmwasm_std::{Addr, Uint128, Uint256};
use cw_storage_plus::Map;
use std::collections::HashMap;

use crate::msg::RewardConfig;

/// a global map that stores total rewards accumulated per token
pub const CUMULATIVE_REWARDS_PER_TOKEN: Map<String, Uint256> = Map::new("c_r_p_t");

/// A map of user addresses to their pending rewards.
pub const PENDING_REWARDS: Map<Addr, HashMap<String, Uint128>> = Map::new("pending_rewards");

/// A map of user addresses to their rewards per token. In other words, it is the
/// reward per share of voting power that the user has.
pub const USER_REWARD_PER_TOKEN: Map<Addr, HashMap<String, Uint256>> =
    Map::new("user_reward_per_token");

// registered hooks mapping to denoms they are registered for
pub const REGISTERED_HOOKS: Map<Addr, Vec<String>> = Map::new("registered_hook_callers");

/// maps denom str to its reward configuration
pub const REWARD_DENOM_CONFIGS: Map<String, RewardConfig> = Map::new("rdc");
