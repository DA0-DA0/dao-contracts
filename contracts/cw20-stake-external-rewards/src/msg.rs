use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg, Denom};
use cw20_stake::hooks::StakeChangedHookMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, RewardConfig};
pub use cw_controllers::ClaimsResponse;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub manager: Option<String>,
    pub staking_contract: String,
    pub reward_token: Denom,
    pub reward_duration: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
    Claim {},
    Receive(Cw20ReceiveMsg),
    Fund {},
    UpdateRewardDuration { new_duration: u64 },
    UpdateOwner { new_owner: Option<String> },
    UpdateManager { new_manager: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Fund {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Info {},
    GetPendingRewards { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoResponse {
    pub config: Config,
    pub reward: RewardConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingRewardsResponse {
    pub address: String,
    pub pending_rewards: Uint128,
    pub denom: Denom,
    pub last_update_block: u64,
}
