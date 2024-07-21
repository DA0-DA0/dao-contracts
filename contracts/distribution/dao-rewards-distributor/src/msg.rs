use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg, UncheckedDenom};
use cw4::MemberChangedHookMsg;
use cw_ownable::cw_ownable_execute;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};
use dao_interface::voting::InfoResponse;

// so that consumers don't need a cw_ownable or cw_controllers dependency
// to consume this contract's queries.
pub use cw_controllers::ClaimsResponse;
pub use cw_ownable::Ownership;

use crate::state::{DenomRewardState, RewardEmissionRate};

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
    Register(RegisterMsg),
    /// updates the config for a registered denom
    Update {
        /// denom to update
        denom: String,
        /// reward emission rate
        emission_rate: Option<RewardEmissionRate>,
        /// whether or not reward distribution is continuous: whether rewards
        /// should be paused once all funding has been distributed, or if future
        /// funding after distribution finishes should be applied to the past.
        continuous: Option<bool>,
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
pub struct RegisterMsg {
    /// denom to register
    pub denom: UncheckedDenom,
    /// reward emission rate
    pub emission_rate: RewardEmissionRate,
    /// whether or not reward distribution is continuous: whether rewards should
    /// be paused once all funding has been distributed, or if future funding
    /// after distribution finishes should be applied to the past.
    pub continuous: bool,
    /// address to query the voting power
    pub vp_contract: String,
    /// address that will update the reward split when the voting power
    /// distribution changes
    pub hook_caller: String,
    /// destination address for reward clawbacks. defaults to owner
    pub withdraw_destination: Option<String>,
}

#[cw_serde]
pub enum MigrateMsg {}

#[cw_serde]
pub enum ReceiveCw20Msg {
    /// Used to fund this contract with cw20 tokens.
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    /// Returns the state of all the registered reward distributions.
    #[returns(RewardsStateResponse)]
    RewardsState {},
    /// Returns the pending rewards for the given address.
    #[returns(PendingRewardsResponse)]
    PendingRewards { address: String },
    /// Returns information about the ownership of this contract.
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    /// Returns the state of the given denom reward distribution.
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
