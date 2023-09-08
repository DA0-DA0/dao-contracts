use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub fee: Option<Vec<Coin>>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Instantiates the target contract with the provided instantiate message and code id and
    /// updates the contract's admin to be itself.
    InstantiateContractWithSelfAdmin {
        instantiate_msg: Binary,
        code_id: u64,
        label: String,
    },
    UpdateFee {
        fee: Option<Vec<Coin>>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Option<Vec<Coin>>)]
    Fee {},
}

#[cw_serde]
pub struct MigrateMsg {}
