use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, BlockInfo, StdError, StdResult, Uint128, Uint256};
use cw20::{Denom, Expiration};
use cw_storage_plus::Map;
use cw_utils::Duration;
use std::{cmp::min, collections::HashMap};

use crate::{msg::RewardEmissionRate, ContractError};

/// map user address to their unique reward state
pub const USER_REWARD_STATES: Map<Addr, UserRewardState> = Map::new("u_r_s");

/// map denom string to the state of its reward distribution
pub const DENOM_REWARD_STATES: Map<String, DenomRewardState> = Map::new("d_r_s");

/// map registered hooks to list of denoms they're registered for
pub const REGISTERED_HOOK_DENOMS: Map<Addr, Vec<String>> = Map::new("r_h_d");

#[cw_serde]
#[derive(Default)]
pub struct UserRewardState {
    /// map denom to the user's pending rewards
    pub pending_denom_rewards: HashMap<String, Uint128>,
    /// map denom string to the user's earned rewards per unit voting power that
    /// have already been accounted for in pending rewards and potentially
    /// claimed
    pub denom_rewards_puvp: HashMap<String, Uint256>,
}

/// the state of a denom's reward distribution
#[cw_serde]
pub struct DenomRewardState {
    /// validated denom (native or cw20)
    pub denom: Denom,
    /// the time when the current reward distribution period started. period
    /// finishes iff it reaches its end.
    pub started_at: Expiration,
    /// the time when all funded rewards are allocated to users and thus the
    /// distribution period ends.
    pub ends_at: Expiration,
    /// reward emission rate
    pub emission_rate: RewardEmissionRate,
    /// total rewards earned per unit voting power from started_at to
    /// last_update
    pub total_earned_puvp: Uint256,
    /// time when total_earned_puvp was last updated for this denom
    pub last_update: Expiration,
    /// address to query the voting power
    pub vp_contract: Addr,
    /// address that will update the reward split when the voting power
    /// distribution changes
    pub hook_caller: Addr,
    /// total amount of rewards funded
    pub funded_amount: Uint128,
    /// optional destination address for reward clawbacks
    pub withdraw_destination: Addr,
}

impl DenomRewardState {
    pub fn bump_last_update(mut self, current_block: &BlockInfo) -> Self {
        self.last_update = match self.emission_rate.duration {
            Duration::Height(_) => Expiration::AtHeight(current_block.height),
            Duration::Time(_) => Expiration::AtTime(current_block.time),
        };
        self
    }

    /// tries to update the last funding date.
    /// if distribution expiration is in the future, nothing changes.
    /// if distribution expiration is in the past, or had never been set,
    /// funding date becomes the current block.
    pub fn bump_funding_date(mut self, current_block: &BlockInfo) -> Self {
        // if its never been set before, we set it to current block and return
        if let Expiration::Never {} = self.started_at {
            self.started_at = match self.emission_rate.duration {
                Duration::Height(_) => Expiration::AtHeight(current_block.height),
                Duration::Time(_) => Expiration::AtTime(current_block.time),
            };
            return self;
        }

        // if current distribution is expired, we set the funding date
        // to the current date
        if self.ends_at.is_expired(current_block) {
            self.started_at = match self.emission_rate.duration {
                Duration::Height(_) => Expiration::AtHeight(current_block.height),
                Duration::Time(_) => Expiration::AtTime(current_block.time),
            };
        }

        self
    }

    pub fn to_str_denom(&self) -> String {
        match &self.denom {
            Denom::Native(denom) => denom.to_string(),
            Denom::Cw20(address) => address.to_string(),
        }
    }

    /// Returns the ends_at time value as a u64.
    /// - If `Never`, returns an error.
    /// - If `AtHeight(h)`, the value is `h`.
    /// - If `AtTime(t)`, the value is `t`, where t is seconds.
    pub fn get_ends_at_scalar(&self) -> StdResult<u64> {
        match self.ends_at {
            Expiration::Never {} => Err(StdError::generic_err("reward period is not active")),
            Expiration::AtHeight(h) => Ok(h),
            Expiration::AtTime(t) => Ok(t.seconds()),
        }
    }

    /// Returns the started_at time value as a u64.
    /// - If `Never`, returns an error.
    /// - If `AtHeight(h)`, the value is `h`.
    /// - If `AtTime(t)`, the value is `t`, where t is seconds.
    pub fn get_started_at_scalar(&self) -> StdResult<u64> {
        match self.started_at {
            Expiration::AtHeight(h) => Ok(h),
            Expiration::AtTime(t) => Ok(t.seconds()),
            Expiration::Never {} => Err(StdError::generic_err("reward period is not active")),
        }
    }

    /// Returns the latest time when rewards were distributed. Works by
    /// comparing `current_block` with the distribution end time:
    /// - If the end is `Never`, then no rewards are being distributed, thus we
    /// return `Never`.
    /// - If the end is `AtHeight(h)` or `AtTime(t)`, we compare the current
    /// block height or time with `h` or `t` respectively.
    /// - If current block respective value is before the end, rewards are still
    /// being distributed. We therefore return the current block `height` or
    /// `time`, as this block is the most recent time rewards were distributed.
    /// - If current block respective value is after the end, rewards are no
    /// longer being distributed. We therefore return the end `height` or
    /// `time`, as that was the last date where rewards were distributed.
    pub fn get_latest_reward_distribution_time(&self, current_block: &BlockInfo) -> Expiration {
        match self.ends_at {
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
        match self.ends_at {
            Expiration::AtHeight(_) | Expiration::AtTime(_) => {
                ensure!(
                    self.ends_at.is_expired(current_block),
                    ContractError::RewardPeriodNotFinished {}
                );
                Ok(())
            }
            Expiration::Never {} => Ok(()),
        }
    }
}
