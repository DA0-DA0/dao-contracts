use cosmwasm_std::{Addr, Uint128};
use cw20::{Cw20ReceiveMsg, Denom};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stake_cw20::hooks::StakeChangedHookMsg;

use cw_utils::Duration;

use crate::state::{Config, RewardConfig};
pub use cw_controllers::ClaimsResponse;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub admin: Option<Addr>,
    pub staking_contract: Addr,
    pub reward_token: Denom,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
    Claim {},
    Receive(Cw20ReceiveMsg),
    Fund {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Fund {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Info {},
    GetPendingRewards { address: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoResponse {
    pub config: Config,
    pub reward: RewardConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingRewardsResponse {
    pub address: Addr,
    pub pending_rewards: Uint128,
    pub denom: Denom,
}
