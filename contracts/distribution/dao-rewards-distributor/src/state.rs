use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, Addr, BlockInfo, Decimal, Deps, StdError, StdResult, Timestamp, Uint128, Uint256,
    Uint64,
};
use cw20::{Denom, Expiration};
use cw_storage_plus::Map;
use cw_utils::Duration;
use std::{cmp::min, collections::HashMap};

use crate::{helpers::get_exp_diff, rewards::get_active_total_earned_puvp, ContractError};

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
    /// have already been accounted for (added to pending and maybe claimed).
    pub accounted_denom_rewards_puvp: HashMap<String, Uint256>,
}

/// defines how many tokens (amount) should be distributed per amount of time
/// (duration). e.g. 5udenom per hour.
#[cw_serde]
pub struct RewardEmissionRate {
    /// amount of tokens to distribute per amount of time
    pub amount: Uint128,
    /// duration of time to distribute amount
    pub duration: Duration,
}

impl RewardEmissionRate {
    // find the duration of the funded period given funded amount. e.g. if the
    // funded amount is twice the emission rate amount, the funded period should
    // be twice the emission rate duration, since the funded amount takes two
    // emission cycles to be distributed.
    pub fn get_funded_period_duration(&self, funded_amount: Uint128) -> StdResult<Duration> {
        // if amount being distributed is 0 (rewards are paused), we return the max duration
        if self.amount.is_zero() {
            return match self.duration {
                Duration::Height(_) => Ok(Duration::Height(u64::MAX)),
                Duration::Time(_) => Ok(Duration::Time(u64::MAX)),
            };
        }

        let amount_to_emission_rate_ratio = Decimal::from_ratio(funded_amount, self.amount);

        let funded_duration = match self.duration {
            Duration::Height(h) => {
                let duration_height = Uint128::from(h)
                    .checked_mul_floor(amount_to_emission_rate_ratio)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;
                let duration = Uint64::try_from(duration_height)?.u64();
                Duration::Height(duration)
            }
            Duration::Time(t) => {
                let duration_time = Uint128::from(t)
                    .checked_mul_floor(amount_to_emission_rate_ratio)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;
                let duration = Uint64::try_from(duration_time)?.u64();
                Duration::Time(duration)
            }
        };

        Ok(funded_duration)
    }
}

#[cw_serde]
pub struct Epoch {
    /// reward emission rate
    pub emission_rate: RewardEmissionRate,
    /// the time when the current reward distribution period started. period
    /// finishes iff it reaches its end.
    pub started_at: Expiration,
    /// the time when all funded rewards are allocated to users and thus the
    /// distribution period ends.
    pub ends_at: Expiration,
    /// total rewards earned per unit voting power from started_at to
    /// last_updated_total_earned_puvp
    pub total_earned_puvp: Uint256,
    /// time when total_earned_puvp was last updated
    pub last_updated_total_earned_puvp: Expiration,
}

impl Epoch {
    /// get the total rewards to be distributed based on the emission rate and
    /// duration from start to end
    pub fn get_total_rewards(&self) -> StdResult<Uint128> {
        let epoch_duration = get_exp_diff(&self.ends_at, &self.started_at)?;

        let emission_rate_duration_scalar = match self.emission_rate.duration {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        };

        self.emission_rate
            .amount
            .checked_multiply_ratio(epoch_duration, emission_rate_duration_scalar)
            .map_err(|e| StdError::generic_err(e.to_string()))
    }

    /// bump the last_updated_total_earned_puvp field to the minimum of the
    /// current block and ends_at since rewards cannot be distributed after
    /// ends_at. this is necessary in the case that a future funding backfills
    /// rewards after they've finished distributing. in order to compute over
    /// the missed space, last_updated can never be greater than ends_at.
    pub fn bump_last_updated(&mut self, current_block: &BlockInfo) -> StdResult<()> {
        match (self.emission_rate.duration, self.ends_at) {
            (Duration::Height(_), Expiration::AtHeight(ends_at_height)) => {
                self.last_updated_total_earned_puvp =
                    Expiration::AtHeight(std::cmp::min(current_block.height, ends_at_height));
                Ok(())
            }
            (Duration::Time(_), Expiration::AtTime(ends_at_time)) => {
                self.last_updated_total_earned_puvp =
                    Expiration::AtTime(std::cmp::min(current_block.time, ends_at_time));
                Ok(())
            }
            _ => Err(StdError::generic_err(
                "Mismatched emission_rate and ends_at block/time units",
            )),
        }
    }
}

/// the state of a denom's reward distribution
#[cw_serde]
pub struct DenomRewardState {
    /// validated denom (native or cw20)
    pub denom: Denom,
    /// current denom distribution epoch state
    pub active_epoch: Epoch,
    /// whether or not reward distribution is continuous: whether rewards should
    /// be paused once all funding has been distributed, or if future funding
    /// after distribution finishes should be applied to the past.
    pub continuous: bool,
    /// address to query the voting power
    pub vp_contract: Addr,
    /// address that will update the reward split when the voting power
    /// distribution changes
    pub hook_caller: Addr,
    /// total amount of rewards funded that will be distributed in the active
    /// epoch.
    pub funded_amount: Uint128,
    /// destination address for reward clawbacks
    pub withdraw_destination: Addr,
    /// historical rewards earned per unit voting power from past epochs due to
    /// changes in the emission rate. each time emission rate is changed, this
    /// value is increased by the `active_epoch`'s rewards earned puvp.
    pub historical_earned_puvp: Uint256,
}

impl DenomRewardState {
    pub fn to_str_denom(&self) -> String {
        match &self.denom {
            Denom::Native(denom) => denom.to_string(),
            Denom::Cw20(address) => address.to_string(),
        }
    }

    /// Returns the latest time when rewards were distributed. Works by
    /// comparing `current_block` with the distribution end time:
    /// - If the end is `Never`, then no rewards are currently being
    ///   distributed, so return the last update.
    /// - If the end is `AtHeight(h)` or `AtTime(t)`, we compare the current
    ///   block height or time with `h` or `t` respectively.
    /// - If current block respective value is before the end, rewards are still
    ///   being distributed. We therefore return the current block `height` or
    ///   `time`, as this block is the most recent time rewards were
    ///   distributed.
    /// - If current block respective value is after the end, rewards are no
    ///   longer being distributed. We therefore return the end `height` or
    ///   `time`, as that was the last date where rewards were distributed.
    pub fn get_latest_reward_distribution_time(&self, current_block: &BlockInfo) -> Expiration {
        match self.active_epoch.ends_at {
            Expiration::Never {} => self.active_epoch.last_updated_total_earned_puvp,
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
        match self.active_epoch.ends_at {
            Expiration::AtHeight(_) | Expiration::AtTime(_) => {
                ensure!(
                    self.active_epoch.ends_at.is_expired(current_block),
                    ContractError::RewardPeriodNotFinished {}
                );
                Ok(())
            }
            Expiration::Never {} => Ok(()),
        }
    }

    /// Finish current epoch early and start a new one with a new emission rate.
    pub fn transition_epoch(
        &mut self,
        deps: Deps,
        new_emission_rate: RewardEmissionRate,
        current_block: &BlockInfo,
    ) -> StdResult<()> {
        // if the new emission rate is the same as the active one, do nothing
        if self.active_epoch.emission_rate == new_emission_rate {
            return Ok(());
        }

        // 1. finish current epoch by updating rewards and setting end to now
        self.active_epoch.total_earned_puvp =
            get_active_total_earned_puvp(deps, current_block, self)?;
        self.active_epoch.ends_at = match self.active_epoch.emission_rate.duration {
            Duration::Height(_) => Expiration::AtHeight(current_block.height),
            Duration::Time(_) => Expiration::AtTime(current_block.time),
        };

        // 2. add current epoch rewards earned to historical rewards
        // TODO: what to do on overflow?
        self.historical_earned_puvp = self
            .historical_earned_puvp
            .checked_add(self.active_epoch.total_earned_puvp)?;

        // 3. deduct the distributed rewards amount from total funded amount, as
        // those rewards are no longer distributed in the new epoch
        let active_epoch_earned_rewards = match self.active_epoch.emission_rate.amount.is_zero() {
            true => Uint128::zero(),
            false => self.active_epoch.get_total_rewards()?,
        };
        self.funded_amount = self
            .funded_amount
            .checked_sub(active_epoch_earned_rewards)?;

        // 4. start new epoch

        // we get the duration of the funded period and add it to the current
        // block height. if the sum overflows, we return u64::MAX, as it
        // suggests that the period is infinite or so long that it doesn't
        // matter.
        let new_ends_at = match new_emission_rate.get_funded_period_duration(self.funded_amount)? {
            Duration::Height(h) => {
                if current_block.height.checked_add(h).is_some() {
                    Expiration::AtHeight(current_block.height + h)
                } else {
                    Expiration::AtHeight(u64::MAX)
                }
            }
            Duration::Time(t) => {
                if current_block.time.seconds().checked_add(t).is_some() {
                    Expiration::AtTime(current_block.time.plus_seconds(t))
                } else {
                    Expiration::AtTime(Timestamp::from_seconds(u64::MAX))
                }
            }
        };

        let new_started_at = match new_emission_rate.duration {
            Duration::Height(_) => Expiration::AtHeight(current_block.height),
            Duration::Time(_) => Expiration::AtTime(current_block.time),
        };

        self.active_epoch = Epoch {
            emission_rate: new_emission_rate.clone(),
            started_at: new_started_at,
            ends_at: new_ends_at,
            // start the new active epoch with zero rewards earned
            total_earned_puvp: Uint256::zero(),
            last_updated_total_earned_puvp: new_started_at,
        };

        Ok(())
    }
}
