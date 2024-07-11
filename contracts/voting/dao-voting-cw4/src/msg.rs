use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_dao_macros::{active_query, voting_module_query};
use dao_voting::threshold::ActiveThreshold;

#[cw_serde]
pub enum GroupContract {
    Existing {
        address: String,
    },
    New {
        cw4_group_code_id: u64,
        initial_members: Vec<cw4::Member>,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub group_contract: GroupContract,
    pub active_threshold: Option<ActiveThreshold>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Sets the active threshold to a new value. Only the
    /// instantiator of this contract (a DAO most likely) may call this
    /// method.
    UpdateActiveThreshold {
        new_threshold: Option<ActiveThreshold>,
    },
}

#[active_query]
#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    GroupContract {},
}

#[cw_serde]
pub struct MigrateMsg {}
