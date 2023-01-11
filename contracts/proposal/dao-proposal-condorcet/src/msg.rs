use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};
use dao_voting::threshold::PercentageThreshold;

#[cw_serde]
pub struct InstantiateMsg {
    quorum: PercentageThreshold,
}

#[cw_serde]
pub struct Choice {
    msgs: Vec<CosmosMsg<Empty>>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Propose { choices: Vec<Choice> },
    Vote { proposal_id: u32, vote: Vec<u32> },
    Execute { proposal_id: u32 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct MigrateMsg {}
