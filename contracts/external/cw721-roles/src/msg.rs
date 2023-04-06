use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::CustomMsg;

#[cw_serde]
pub struct MetadataExt {
    pub weight: u32,
}

#[cw_serde]
pub enum ExecuteExt {
    /// Add a new hook to be informed of all membership changes.
    /// Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
}
impl CustomMsg for ExecuteExt {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    /// Total weight at a given height
    #[returns(cw4::TotalWeightResponse)]
    TotalWeight { at_height: Option<u64> },
    /// Returns the weight of a certain member
    #[returns(cw4::MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(cw_controllers::HooksResponse)]
    Hooks {},
}
impl CustomMsg for QueryExt {}
