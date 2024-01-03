use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg, Denom};
use cw_ownable::cw_ownable_execute;
use dao_hooks::stake::StakeChangedHookMsg;

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
    /// An optional staking contract that is allowed to call the StakeChangedHook.
    /// Often, this is the same as the vp_contract, but sometimes they are separate.
    pub staking_contract: Option<String>,
    /// The Denom in which rewards are paid out.
    pub reward_token: Denom,
    /// The duration of the reward period in blocks.
    pub reward_duration: u64,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
    Claim {},
    Receive(Cw20ReceiveMsg),
    Fund {},
    UpdateRewardDuration { new_duration: u64 },
}

#[cw_serde]
pub enum MigrateMsg {}

#[cw_serde]
pub enum ReceiveMsg {
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(InfoResponse)]
    Info {},
    #[returns(PendingRewardsResponse)]
    GetPendingRewards { address: String },
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
