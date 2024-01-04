use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg, Denom};
use cw4::MemberChangedHookMsg;
use cw_ownable::cw_ownable_execute;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};

use crate::state::{Config, RewardConfig};

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
    /// An optional contract that is allowed to call the StakeChangedHook.
    /// Often, the vp_contract calls hooks for power change events, but sometimes
    /// they are separate. For example, the cw20-stake contract is separate from
    /// the dao-voting-cw20-staked contract.
    pub hook_caller: Option<String>,
    /// The denom in which rewards are paid out.
    pub reward_denom: Denom,
    /// The duration of the reward period in blocks.
    pub reward_duration: u64,
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
    Claim {},
    /// Used to fund this contract with cw20 tokens.
    Receive(Cw20ReceiveMsg),
    /// Used to fund this contract with native tokens.
    Fund {},
    /// Updates the reward duration which controls the rate that rewards are issued.
    UpdateRewardDuration { new_duration: u64 },
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
    pub config: Config,
    pub reward: RewardConfig,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub address: String,
    pub pending_rewards: Uint128,
    pub denom: Denom,
    pub last_update_block: u64,
}
