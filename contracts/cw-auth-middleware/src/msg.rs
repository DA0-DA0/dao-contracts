use cosmwasm_std::{CosmosMsg, Empty};
use cw_core::msg::ModuleInstantiateInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Instantiate information for the proposal contract's  that
    /// this authorization middlware is proxying
    pub proposal_module_instantiate_info: ModuleInstantiateInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAuthorization { auth_contract: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAuthorizations {},
    Authorize {
        msgs: Vec<CosmosMsg<Empty>>,
        sender: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsAuthorizedResponse {
    pub authorized: bool,
}
