use cosmwasm_std::Uint128;
use cw_governance_macros::cw_governance_voting_query;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_governance_voting_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Query {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VotingPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TotalPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}
