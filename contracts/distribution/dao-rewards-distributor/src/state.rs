use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, BlockInfo, Uint128, Uint256};
use cw20::{Denom, Expiration};
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;
use std::cmp::min;

use crate::ContractError;

#[cw_serde]
pub struct Config {
    /// The address of a DAO DAO voting power module contract.
    pub vp_contract: Addr,
    /// An optional contract that is allowed to call the StakeChangedHook in
    /// place of the voting power contract.
    pub hook_caller: Option<Addr>,
    /// The Denom in which rewards are paid out.
    pub reward_denom: Denom,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct RewardConfig {
    pub period_finish_expiration: Expiration,
    pub reward_rate: Uint128,
    pub reward_duration: Duration,
}

impl RewardConfig {
    /// Returns the reward duration value as a u64.
    /// If the reward duration is in blocks, the value is the number of blocks.
    /// If the reward duration is in time, the value is the number of seconds.
    pub fn get_reward_duration_value(&self) -> u64 {
        match self.reward_duration {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        }
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
        match self.period_finish_expiration {
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
        match self.period_finish_expiration {
            Expiration::AtHeight(_) | Expiration::AtTime(_) => {
                ensure!(
                    self.period_finish_expiration.is_expired(current_block),
                    ContractError::RewardPeriodNotFinished {}
                );
                Ok(())
            }
            Expiration::Never {} => Ok(()),
        }
    }
}

pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");

pub const REWARD_PER_TOKEN: Item<Uint256> = Item::new("reward_per_token");

pub const LAST_UPDATE_EXPIRATION: Item<Expiration> = Item::new("last_update_snapshot");

/// A map of user addresses to their pending rewards.
pub const PENDING_REWARDS: Map<Addr, Uint128> = Map::new("pending_rewards");

/// A map of user addresses to their rewards per token. In other words, it is the
/// reward per share of voting power that the user has.
pub const USER_REWARD_PER_TOKEN: Map<Addr, Uint256> = Map::new("user_reward_per_token");
