use cosmwasm_std::{Addr, CosmosMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Kind;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub dao: Addr,
    pub kind: Kind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAuthorization { addr: Addr, msg: String },
    RemoveAuthorization { addr: Addr, msg: String },
    Authorize { msgs: Vec<CosmosMsg>, sender: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {}
