use std::{cmp::min, collections::HashMap};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, BlockInfo, Uint128};
use cw20::{Cw20ReceiveMsg, Denom, UncheckedDenom};
use cw4::MemberChangedHookMsg;
use cw_ownable::cw_ownable_execute;
use cw_utils::{Duration, Expiration};
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};

use crate::ContractError;

// so that consumers don't need a cw_ownable or cw_controllers dependency
// to consume this contract's queries.
pub use cw_controllers::ClaimsResponse;
pub use cw_ownable::Ownership;

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the contract. Is able to fund the contract and update
    /// the reward duration.
    pub owner: Option<String>,
    /// A DAO DAO voting power module contract address.
    pub vp_contract: String,
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
    /// Updates the reward duration which controls the rate that rewards are issued.
    UpdateRewardDuration {
        new_duration: Duration,
        denom: String,
    },
    /// shuts down the rewards distributor. withdraws all future staking rewards
    /// back to the treasury. members can claim whatever they earned until this point.
    Shutdown { denom: String },
    /// registers a new reward denom
    RegisterRewardDenom(RewardDenomRegistrationMsg),
}

#[cw_serde]
pub struct RewardDenomRegistrationMsg {
    pub denom: UncheckedDenom,
    pub reward_rate: Uint128,
    pub reward_duration: Duration,
    pub hook_caller: Option<String>,
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
    /// Returns configuration information about this contract.
    #[returns(InfoResponse)]
    Info {},
    /// Returns the pending rewards for the given address.
    #[returns(PendingRewardsResponse)]
    GetPendingRewards { address: String },
    /// Returns information about the ownership of this contract.
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
}

#[cw_serde]
pub struct InfoResponse {
    pub vp_contract: String,
    pub reward_configs: Vec<RewardConfig>,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub address: String,
    pub pending_rewards: HashMap<String, Uint128>,
}

/// a config that holds info needed to distribute rewards
#[cw_serde]
pub struct RewardConfig {
    /// expiration snapshot for the current period
    pub period_finish_expiration: Expiration,
    /// validated denom (native/cw20)
    pub denom: Denom,
    pub reward_rate: Uint128,
    /// time or block based duration to be used for reward distribution
    pub reward_duration: Duration,
    /// last update date
    pub last_update: Expiration,
    // pub hook_caller: String,
    pub funded_amount: Uint128,
}

impl RewardConfig {
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
        match self.reward_duration {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        }
    }

    /// Returns the period finish expiration value as a u64.
    /// If the period finish expiration is `Never`, the value is 0.
    /// If the period finish expiration is `AtHeight(h)`, the value is `h`.
    /// If the period finish expiration is `AtTime(t)`, the value is `t`, where t is seconds.
    pub fn get_period_finish_units(&self) -> u64 {
        match self.period_finish_expiration {
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
