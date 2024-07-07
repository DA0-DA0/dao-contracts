use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, BlockInfo, StdError, StdResult, Timestamp, Uint128, Uint256};
use cw20::{Denom, Expiration};
use cw_storage_plus::Map;
use cw_utils::Duration;
use std::{cmp::min, collections::HashMap};

use crate::{contract::get_start_end_diff, msg::RewardEmissionRate, ContractError};

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

#[cw_serde]
pub struct EpochConfig {
    /// reward emission rate
    pub emission_rate: RewardEmissionRate,
    /// the time when the current reward distribution period started. period
    /// finishes iff it reaches its end.
    pub started_at: Expiration,
    /// the time when all funded rewards are allocated to users and thus the
    /// distribution period ends.
    pub ends_at: Expiration,
    /// total rewards earned per unit voting power from started_at to
    /// last_update
    pub total_earned_puvp: Uint256,
    /// finish block height
    pub finish_height: Option<BlockInfo>,
}

impl EpochConfig {
    pub fn get_total_distributed_rewards(&self) -> Result<Uint128, ContractError> {
        let epoch_duration = get_start_end_diff(&self.started_at, &self.ends_at)?;

        let emission_rate_duration_scalar = match self.emission_rate.duration {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        };

        self.emission_rate
            .amount
            .checked_multiply_ratio(epoch_duration, emission_rate_duration_scalar)
            .map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))
    }
}

/// the state of a denom's reward distribution
#[cw_serde]
pub struct DenomRewardState {
    /// validated denom (native or cw20)
    pub denom: Denom,
    /// current denom distribution epoch configuration
    pub active_epoch_config: EpochConfig,
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
    /// historic denom distribution epochs
    pub historic_epoch_configs: Vec<EpochConfig>,
}

impl DenomRewardState {
    pub fn transition_epoch(
        &mut self,
        new_emission_rate: RewardEmissionRate,
        current_block: &BlockInfo,
    ) -> StdResult<()> {
        let current_block_scalar = match self.active_epoch_config.emission_rate.duration {
            Duration::Height(_) => Expiration::AtHeight(current_block.height),
            Duration::Time(_) => Expiration::AtTime(current_block.time),
        };

        // 1. finish current epoch
        let mut curr_epoch = self.active_epoch_config.clone();
        curr_epoch.ends_at = current_block_scalar;
        curr_epoch.finish_height = Some(current_block.to_owned());

        println!("transition_epoch: {:?}", curr_epoch);
        // 2. push current epoch to historic configs
        self.historic_epoch_configs.push(curr_epoch.clone());

        // 3. deduct the distributed rewards amount from total funded amount,
        // as those rewards are no longer available for distribution
        let curr_epoch_earned_rewards = match curr_epoch.emission_rate.amount.is_zero() {
            true => Uint128::zero(),
            false => self
                .active_epoch_config
                .get_total_distributed_rewards()
                .map_err(|e| StdError::generic_err(e.to_string()))?,
        };

        self.funded_amount = self.funded_amount.checked_sub(curr_epoch_earned_rewards)?;
        // 4. start new epoch
        println!("fund amount: {:?}", self.funded_amount);
        println!("new_emission_rate: {:?}", new_emission_rate);
        let new_epoch_end_scalar =
            self.calculate_ends_at(&new_emission_rate, self.funded_amount, current_block)?;
        self.active_epoch_config = EpochConfig {
            emission_rate: new_emission_rate.clone(),
            started_at: current_block_scalar,
            ends_at: new_epoch_end_scalar,
            // start the new active epoch with zero puvp
            total_earned_puvp: Uint256::zero(),
            finish_height: None,
        };

        Ok(())
    }

    fn calculate_ends_at(
        &self,
        emission_rate: &RewardEmissionRate,
        funded_amount: Uint128,
        current_block: &BlockInfo,
    ) -> StdResult<Expiration> {
        // we get the duration of the funded period and add it to the current
        // block height. if the sum overflows, we return u64::MAX, as it suggests
        // that the period is infinite or so long that it doesn't matter.
        match emission_rate.get_funded_period_duration(funded_amount)? {
            Duration::Height(h) => {
                if current_block.height.checked_add(h).is_some() {
                    Ok(Expiration::AtHeight(current_block.height + h))
                } else {
                    Ok(Expiration::AtHeight(u64::MAX))
                }
            }
            Duration::Time(t) => {
                if current_block.time.seconds().checked_add(t).is_some() {
                    Ok(Expiration::AtTime(current_block.time.plus_seconds(t)))
                } else {
                    Ok(Expiration::AtTime(Timestamp::from_seconds(u64::MAX)))
                }
            }
        }
    }
}

impl DenomRewardState {
    pub fn bump_last_update(mut self, current_block: &BlockInfo) -> Self {
        self.last_update = match self.active_epoch_config.emission_rate.duration {
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
        if let Expiration::Never {} = self.active_epoch_config.started_at {
            self.active_epoch_config.started_at =
                match self.active_epoch_config.emission_rate.duration {
                    Duration::Height(_) => Expiration::AtHeight(current_block.height),
                    Duration::Time(_) => Expiration::AtTime(current_block.time),
                };
            return self;
        }

        // if current distribution is expired, we set the funding date
        // to the current date
        if self.active_epoch_config.ends_at.is_expired(current_block) {
            self.active_epoch_config.started_at =
                match self.active_epoch_config.emission_rate.duration {
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
        match self.active_epoch_config.ends_at {
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
        match self.active_epoch_config.started_at {
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
        match self.active_epoch_config.ends_at {
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
        match self.active_epoch_config.ends_at {
            Expiration::AtHeight(_) | Expiration::AtTime(_) => {
                ensure!(
                    self.active_epoch_config.ends_at.is_expired(current_block),
                    ContractError::RewardPeriodNotFinished {}
                );
                Ok(())
            }
            Expiration::Never {} => Ok(()),
        }
    }
}
