use cosmwasm_schema::{cw_serde};

#[cw_serde]
pub struct InstantiateMsg {
    // To determine voting power
    pub voting_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
pub enum QueryMsg {}
