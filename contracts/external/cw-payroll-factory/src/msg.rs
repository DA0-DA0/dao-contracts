use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_payroll::msg::InstantiateMsg as PayrollInstantiateMsg;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Instantiates the target contract with the provided instantiate message and code id and
    /// updates the contract's admin to be itself.
    InstantiatePayrollContract {
        instantiate_msg: PayrollInstantiateMsg,
        code_id: u64,
        label: String,
    },
}

// TODO recieve method for instanitating with a cw20?

// TODO get vesting_payments by recipient or instantiator
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // #[returns(Vec<crate::state::VestingPayment>)]
    // ListVestingContracts {
    //     instantiator: Option<String>,
    //     start_after: Option<u64>,
    //     limit: Option<u32>,
    // },
}

#[cw_serde]
pub struct MigrateMsg {}
