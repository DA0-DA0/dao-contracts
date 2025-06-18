use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

pub use cw_ownable::Ownership;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the initial owner of the contract. Defaults to the
    /// instantiator.
    pub owner: Option<String>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct AdminResponse {
    pub admin: Option<Addr>,
}
