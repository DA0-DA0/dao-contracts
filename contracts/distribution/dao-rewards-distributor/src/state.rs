use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, BlockInfo, Uint128, Uint256};
use cw20::{Denom, Expiration};
use cw_storage_plus::Map;
use cw_utils::Duration;
use std::{cmp::min, collections::HashMap};

use crate::{msg::RewardEmissionConfig, ContractError};

/// maps user address to their unique reward configuration
pub const USER_REWARD_CONFIGS: Map<Addr, UserRewardConfig> = Map::new("user_reward_configs");

/// a global map that stores total rewards accumulated per token
pub const CUMULATIVE_REWARDS_PER_TOKEN: Map<String, Uint256> = Map::new("c_r_p_t");

// registered hooks mapping to denoms they are registered for
pub const REGISTERED_HOOKS: Map<Addr, Vec<String>> = Map::new("registered_hook_callers");

/// maps denom str to its reward configuration
pub const REWARD_DENOM_CONFIGS: Map<String, DenomRewardConfig> = Map::new("r_d_c");

#[cw_serde]
#[derive(Default)]
pub struct UserRewardConfig {
    pub pending_denom_rewards: HashMap<String, Uint128>,
    pub user_reward_per_token: HashMap<String, Uint256>,
}

/// a config that holds info needed to distribute rewards
#[cw_serde]
pub struct DenomRewardConfig {
    /// time until all funded rewards are allocated to users
    pub distribution_expiration: Expiration,
    /// validated denom (native/cw20)
    pub denom: Denom,
    /// config determining reward distribution rate
    /// per specified duration
    pub reward_emission_config: RewardEmissionConfig,
    /// last update date
    pub last_update: Expiration,
    pub hook_caller: Addr,
    pub vp_contract: Addr,
    pub funded_amount: Uint128,
}

impl DenomRewardConfig {
    pub fn bump_last_update(mut self, current_block: &BlockInfo) -> Self {
        self.last_update = match self.reward_emission_config.reward_rate_time {
            Duration::Height(_) => Expiration::AtHeight(current_block.height),
            Duration::Time(_) => Expiration::AtTime(current_block.time),
        };
        self
    }

    pub fn to_str_denom(&self) -> String {
        match &self.denom {
            Denom::Native(denom) => denom.to_string(),
            Denom::Cw20(address) => address.to_string(),
        }
    }

    /// Returns the reward duration value as a u64.
    /// If the reward duration is in blocks, the value is the number of blocks.
    /// If the reward duration is in time, the value is the number of seconds.
    pub fn get_reward_duration_value(&self) -> u64 {
        match self.reward_emission_config.reward_rate_time {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        }
    }

    /// Returns the period finish expiration value as a u64.
    /// If the period finish expiration is `Never`, the value is 0.
    /// If the period finish expiration is `AtHeight(h)`, the value is `h`.
    /// If the period finish expiration is `AtTime(t)`, the value is `t`, where t is seconds.
    pub fn get_period_finish_units(&self) -> u64 {
        match self.distribution_expiration {
            Expiration::Never {} => 0,
            Expiration::AtHeight(h) => h,
            Expiration::AtTime(t) => t.seconds(),
        }
    }

    /// Returns the period start date value as a u64.
    /// The period start date is calculated by subtracting the reward duration
    /// value from the period finish expiration value.
    // TODO: ensure this cannot go wrong
    pub fn get_period_start_units(&self) -> u64 {
        let period_finish_units = self.get_period_finish_units();
        let reward_duration_value = self.get_reward_duration_value();
        period_finish_units - reward_duration_value
    }

    /// Returns the latest date where rewards were still being distributed.
    /// Works by comparing `current_block` with the period finish expiration:
    /// - If the period finish expiration is `Never`, then no rewards are being
    /// distributed, thus we return `Never`.
    /// - If the period finish expiration is `AtHeight(h)` or `AtTime(t)`,
    /// we compare the current block height or time with `h` or `t` respectively.
    /// If current block respective value is lesser than that of the
    /// `period_finish_expiration`, means rewards are still being distributed.
    /// We therefore return the current block `height` or `time`, as that was the
    /// last date where rewards were distributed.
    /// If current block respective value is greater than that of the
    /// `period_finish_expiration`, means rewards are no longer being distributed.
    /// We therefore return the `period_finish_expiration` value, as that was the
    /// last date where rewards were distributed.
    pub fn get_latest_reward_distribution_expiration_date(
        &self,
        current_block: &BlockInfo,
    ) -> Expiration {
        match self.distribution_expiration {
            Expiration::Never {} => Expiration::Never {},
            Expiration::AtHeight(h) => Expiration::AtHeight(min(current_block.height, h)),
            Expiration::AtTime(t) => Expiration::AtTime(min(current_block.time, t)),
        }
    }

    /// Returns `ContractError::RewardPeriodNotFinished` if the period finish
    /// expiration is of either `AtHeight` or `AtTime` variant and is earlier
    /// than the current block height or time respectively.
    pub fn validate_period_finish_expiration_if_set(
        &self,
        current_block: &BlockInfo,
    ) -> Result<(), ContractError> {
        match self.distribution_expiration {
            Expiration::AtHeight(_) | Expiration::AtTime(_) => {
                ensure!(
                    self.distribution_expiration.is_expired(current_block),
                    ContractError::RewardPeriodNotFinished {}
                );
                Ok(())
            }
            Expiration::Never {} => Ok(()),
        }
    }
}
