use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use dao_interface::token::InitialBalance;

#[cw_serde]
pub struct InstantiateMsg {
    /// The account allowed to execute this contract. If no admin, anyone can
    /// execute it.
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Issues a new fantoken.
    Issue(NewFanToken),
}

#[cw_serde]
pub struct CreatingFanToken {
    /// Fan token info.
    pub token: NewFanToken,
    /// DAO address.
    pub dao: Addr,
}

#[cw_serde]
pub struct NewFanToken {
    /// Fan token symbol.
    pub symbol: String,
    /// Fan token name.
    pub name: String,
    /// Fan token max supply.
    pub max_supply: Uint128,
    /// Fan token URI.
    pub uri: String,
    /// The initial balances to set for the token, cannot be empty.
    pub initial_balances: Vec<InitialBalance>,
    /// Optional balance to mint for the DAO.
    pub initial_dao_balance: Option<Uint128>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct MigrateMsg {}
