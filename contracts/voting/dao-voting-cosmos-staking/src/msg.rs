use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_dao_macros::{active_query, voting_module_query};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {}

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
pub enum SudoMsg {
    // Follow delegations with Juno's cw-hooks module
    // https://github.com/CosmosContracts/juno/tree/main/x/cw-hooks
    // BeforeDelegationCreated {
    //     validator_address: String,
    //     delegator_address: String,
    //     shares: String,
    // },

    BeforeDelegationSharesModified {
        validator_address: String,
        delegator_address: String,
        shares: String,
    },

    // AfterDelegationModified {
    //     validator_address: String,
    //     delegator_address: String,
    //     shares: String,
    // },

    // BeforeDelegationRemoved {
    //     validator_address: String,
    //     delegator_address: String,
    //     shares: String,
    // },
}


#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}
