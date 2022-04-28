use schemars::JsonSchema;
use cosmwasm_std::{Uint128};
use cw20::Denom;
use serde::{Deserialize, Serialize};
use crate::state::{Config};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub recipient: String,
    pub reward_rate: Uint128,
    pub token: Denom
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        owner: String,
        recipient: String,
        reward_rate: Uint128,
        token: Denom
},
    Distribute {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub config: Config
}

