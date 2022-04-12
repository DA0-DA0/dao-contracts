use cosmwasm_std::Uint128;
use cw20_staked_balance_voting::msg::TokenInfo;
use cw_core_macros::voting_query;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stake_cw20::hooks::StakeChangedHookMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_info: TokenInfo,
    pub initial_dao_balance: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    StakeChangedHook(StakeChangedHookMsg),
}

#[voting_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
