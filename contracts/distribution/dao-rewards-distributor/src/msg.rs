use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128, Uint64};
use cw20::{Cw20ReceiveMsg, UncheckedDenom};
use cw4::MemberChangedHookMsg;
use cw_ownable::cw_ownable_execute;
use cw_utils::Duration;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};
use dao_interface::voting::InfoResponse;

// so that consumers don't need a cw_ownable or cw_controllers dependency
// to consume this contract's queries.
pub use cw_controllers::ClaimsResponse;
pub use cw_ownable::Ownership;

use crate::state::DenomRewardState;

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the contract. Is able to fund the contract and update
    /// the reward duration.
    pub owner: Option<String>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Called when a member is added or removed
    /// to a cw4-groups or cw721-roles contract.
    MemberChangedHook(MemberChangedHookMsg),
    /// Called when NFTs are staked or unstaked.
    NftStakeChangeHook(NftStakeChangedHookMsg),
    /// Called when tokens are staked or unstaked.
    StakeChangeHook(StakeChangedHookMsg),
    /// registers a new reward denom
    RegisterDenom(RegisterDenomMsg),
    /// updates the config for a registered denom
    UpdateDenom {
        /// denom to update
        denom: String,
        /// reward emission rate
        emission_rate: Option<RewardEmissionRate>,
        /// address to query the voting power
        vp_contract: Option<String>,
        /// address that will update the reward split when the voting power
        /// distribution changes
        hook_caller: Option<String>,
        /// destination address for reward clawbacks. defaults to owner
        withdraw_destination: Option<String>,
    },
    /// Used to fund this contract with cw20 tokens.
    Receive(Cw20ReceiveMsg),
    /// Used to fund this contract with native tokens.
    Fund {},
    /// Claims rewards for the sender.
    Claim { denom: String },
    /// withdraws the undistributed rewards for a denom. members can claim
    /// whatever they earned until this point. this is effectively an inverse to
    /// fund and does not affect any already-distributed rewards.
    Withdraw { denom: String },
}

#[cw_serde]
pub struct RegisterDenomMsg {
    /// denom to register
    pub denom: UncheckedDenom,
    /// reward emission rate
    pub emission_rate: RewardEmissionRate,
    /// address to query the voting power
    pub vp_contract: String,
    /// address that will update the reward split when the voting power
    /// distribution changes
    pub hook_caller: String,
    /// destination address for reward clawbacks. defaults to owner
    pub withdraw_destination: Option<String>,
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
pub enum MigrateMsg {}

#[cw_serde]
pub enum ReceiveMsg {
    /// Used to fund this contract with cw20 tokens.
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    /// Returns the state of the registered reward distributions.
    #[returns(RewardsStateResponse)]
    RewardsState {},
    /// Returns the pending rewards for the given address.
    #[returns(PendingRewardsResponse)]
    GetPendingRewards { address: String },
    /// Returns information about the ownership of this contract.
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    #[returns(DenomRewardState)]
    DenomRewardState { denom: String },
}

#[cw_serde]
pub struct RewardsStateResponse {
    pub rewards: Vec<DenomRewardState>,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub address: String,
    pub pending_rewards: HashMap<String, Uint128>,
}
