use cosmwasm_std::{CosmosMsg, Empty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod voting;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ExecuteProposalHook { msgs: Vec<CosmosMsg<Empty>> },
}
