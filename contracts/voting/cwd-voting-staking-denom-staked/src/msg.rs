use cosmwasm_schema::{cw_serde, QueryResponses};
use cwd_macros::voting_module_query;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the chain's staking module, the
    /// balance of this address will be the amount of
    /// staked tokens across the network.
    pub staking_module_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    StakingModule {},
}

#[cw_serde]
pub struct MigrateMsg {}
