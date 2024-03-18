use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};

#[cw_serde]
pub struct InstantiateMsg {
    /// The account allowed to execute this contract.
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Instantiates the target contract with the provided instantiate message and code id and
    /// updates the contract's admin to be itself.
    InstantiateContractWithSelfAdmin {
        instantiate_msg: Binary,
        code_id: u64,
        label: String,
    },
    /// Update the admin that is allowed to execute this contract. If there is
    /// no admin, this cannot be called and there will never be an admin.
    UpdateAdmin { admin: Option<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AdminResponse)]
    Admin {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct AdminResponse {
    pub admin: Option<Addr>,
}
