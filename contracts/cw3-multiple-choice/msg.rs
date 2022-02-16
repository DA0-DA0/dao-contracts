use cosmwasm_std::{CosmosMsg, Empty};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MultipleChoiceProposeMsg {
    pub title: String,
    pub description: String,
    pub choices: Vec<String>,
    pub msgs: Vec<Vec<CosmosMsg<Empty>>>,
    // note: we ignore API-spec'd earliest if passed, always opens immediately
    pub latest: Option<Expiration>,
}
