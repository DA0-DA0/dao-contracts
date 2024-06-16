use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{StdError, StdResult, Uint128, Uint256};
use cw20::{Cw20ReceiveMsg, UncheckedDenom};
use cw4::MemberChangedHookMsg;
use cw_ownable::cw_ownable_execute;
use cw_utils::Duration;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};
use dao_interface::voting::InfoResponse;

use crate::{state::DenomRewardState, ContractError};

// so that consumers don't need a cw_ownable or cw_controllers dependency
// to consume this contract's queries.
pub use cw_controllers::ClaimsResponse;
pub use cw_ownable::Ownership;

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
    /// Claims rewards for the sender.
    Claim { denom: String },
    /// Used to fund this contract with cw20 tokens.
    Receive(Cw20ReceiveMsg),
    /// Used to fund this contract with native tokens.
    Fund {},
    /// shuts down the rewards distributor. withdraws all future staking rewards
    /// back to the treasury. members can claim whatever they earned until this point.
    Shutdown { denom: String },
    /// registers a new reward denom
    RegisterRewardDenom {
        denom: UncheckedDenom,
        emission_rate: RewardEmissionRate,
        vp_contract: String,
        hook_caller: String,
        withdraw_destination: Option<String>,
    },
}

/// defines how many tokens (amount) should be distributed per amount of time
/// (duration). e.g. 5udenom per hour.
#[cw_serde]
pub struct RewardEmissionRate {
    pub amount: Uint128,
    pub duration: Duration,
}

impl RewardEmissionRate {
    pub fn validate_emission_time_window(&self) -> Result<(), ContractError> {
        // Reward duration must be greater than 0
        if let Duration::Height(0) | Duration::Time(0) = self.duration {
            return Err(ContractError::ZeroRewardDuration {});
        }
        Ok(())
    }

    // find the duration of the funded period given emission config and funded amount
    pub fn get_funded_period_duration(&self, funded_amount: Uint128) -> StdResult<Duration> {
        let funded_amount_u256 = Uint256::from(funded_amount);
        let amount_u256 = Uint256::from(self.amount);
        let amount_to_emission_rate_ratio = funded_amount_u256.checked_div(amount_u256)?;

        let ratio_str = amount_to_emission_rate_ratio.to_string();
        let ratio = ratio_str
            .parse::<u64>()
            .map_err(|e| StdError::generic_err(e.to_string()))?;

        let funded_period_duration = match self.duration {
            Duration::Height(h) => {
                let duration_height = match ratio.checked_mul(h) {
                    Some(duration) => duration,
                    None => return Err(StdError::generic_err("overflow")),
                };
                Duration::Height(duration_height)
            }
            Duration::Time(t) => {
                let duration_time = match ratio.checked_mul(t) {
                    Some(duration) => duration,
                    None => return Err(StdError::generic_err("overflow")),
                };
                Duration::Time(duration_time)
            }
        };

        Ok(funded_period_duration)
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
