use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    /// The account allowed to execute this contract. If no admin, anyone can
    /// execute it.
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Issues a new fantoken.
    Issue {
        symbol: String,
        name: String,
        max_supply: Uint128,
        authority: String,
        minter: String,
        uri: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct MigrateMsg {}
