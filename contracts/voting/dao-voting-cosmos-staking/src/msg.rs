use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128};
use cw_utils::Duration;
use dao_dao_macros::{active_query, voting_module_query};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Updates the contract configuration
    UpdateConfig { duration: Option<Duration> },
    /// Adds a hook that fires on staking / unstaking
    AddHook { addr: String },
    /// Removes a hook that fires on staking / unstaking
    RemoveHook { addr: String },
}

#[active_query]
#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(GetHooksResponse)]
    GetHooks {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}
