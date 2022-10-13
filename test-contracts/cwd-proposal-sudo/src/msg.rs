use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CosmosMsg};
use cwd_interface::voting::InfoResponse;

use cwd_macros::{info_query, proposal_module_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub root: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Execute { msgs: Vec<CosmosMsg> },
}

#[proposal_module_query]
#[info_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    Admin {},
}
