use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg, Denom};
use cw20_stake::hooks::StakeChangedHookMsg;

use crate::state::{Config, RewardConfig};
pub use cw_controllers::ClaimsResponse;

use cw_ownable::cw_ownable;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub staking_contract: String,
    pub reward_token: Denom,
    pub reward_duration: u64,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_ownable]
#[cw_serde]
pub enum ExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
    Claim {},
    Receive(Cw20ReceiveMsg),
    Fund {},
    UpdateRewardDuration { new_duration: u64 },
}

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
