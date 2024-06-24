use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_dao_macros::voting_module_query;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {}

#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
