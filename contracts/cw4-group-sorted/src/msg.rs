use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::query::Member;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The admin has the ability to add new items to the member list
    /// and to update the admin. Omit the admin to make the member
    /// list immutable.
    pub admin: Option<String>,
    /// The members to initialize the contract with.
    pub members: Vec<Member>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: Option<String> },
    /// Applys a diff to the existing members. Remove is applied
    /// after add, so if an address is in both, it is removed
    UpdateMembers {
        remove: Vec<String>,
        add: Vec<Member>,
    },
    /// Add a new hook to be informed of all membership changes. Must
    /// be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the admin of the contract. Returns `AdminResponse`.
    Admin {},
    /// Get the total weight of member items. Returns
    /// `TotalWeightResponse`.
    TotalWeight {},
    /// Returns `MembersListResponse`. Members are returned in sorted
    /// order with the highest weight members returned first.
    ListMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get information about a member. Optionally, get information
    /// about a member at a given block height. Returns
    /// `MemberResponse`.
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Get all registered hooks. Returns `HooksResponse`.
    Hooks {},
}
