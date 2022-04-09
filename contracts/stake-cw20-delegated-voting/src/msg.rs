use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg, Denom};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stake_cw20::hooks::StakeChangedHookMsg;

pub use stake_cw20::msg::StakedBalanceAtHeightResponse;


#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub staking_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
    Delegate{address: String},
    Undelegate{}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    Delegation {
        address: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DelegationResponse {
    pub address: String,
}