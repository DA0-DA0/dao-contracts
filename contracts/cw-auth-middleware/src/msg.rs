use cosmwasm_std::{Addr, CosmosMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAuthorization {
        auth_contract: String,
    },
    RemoveAuthorization {
        auth_contract: Addr,
    },
    /// Some authorizations may want to track information about the users or
    /// messages to determine if they authorize or not. This message should be
    /// sent every time the authorizations are successfully used so that
    /// sub-authorizations can update their internal state.
    UpdateExecutedAuthorizationState {
        msgs: Vec<CosmosMsg>,
        sender: Addr,
    },
    // This contract can act as a proposal for a dao. This message allows a sender to
    // execute the messages through proposal.
    Execute {
        msgs: Vec<CosmosMsg>,
    },
    ReplaceOwner {
        new_dao: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAuthorizations {},
    Authorize { msgs: Vec<CosmosMsg>, sender: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsAuthorizedResponse {
    pub authorized: bool,
}
