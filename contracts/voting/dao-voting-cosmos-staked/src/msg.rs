use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use dao_dao_macros::voting_module_query;

#[cw_serde]
pub struct InstantiateMsg {
    /// Total staked balance to start with.
    pub total_staked: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set the total staked balance at a given height or the current height.
    UpdateTotalStaked {
        amount: Uint128,
        height: Option<u64>,
    },
}

#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct MigrateMsg {}
