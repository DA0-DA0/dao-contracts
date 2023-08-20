use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};

use dao_dao_macros::proposal_module_query;

use crate::config::UncheckedConfig;

pub type InstantiateMsg = UncheckedConfig;

#[cw_serde]
pub struct Choice {
    pub msgs: Vec<CosmosMsg<Empty>>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Propose { choices: Vec<Choice> },
    Vote { proposal_id: u32, vote: Vec<u32> },
    Execute { proposal_id: u32 },
    Close { proposal_id: u32 },
    SetConfig(UncheckedConfig),
}

#[proposal_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::proposal::ProposalResponse)]
    Proposal { id: u32 },
    #[returns(crate::config::Config)]
    Config {},
}
