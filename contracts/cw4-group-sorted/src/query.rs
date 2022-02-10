use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use cw4::{AdminResponse, HooksResponse, MemberResponse, TotalWeightResponse};

/// The response returned by `QueryMsg::ListMembers`. We redeclare it
/// instead of using the cw4 version as we use our own member type.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MemberListResponse {
    pub members: Vec<Member>,
}

/// An address and its weight. Items are ordered first by their
/// priority and compared based on their addresses.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Member {
    pub addr: Addr,
    pub weight: u64,
}

// Member is an interesting type. Two members are equal if they
// represent the same address. Members are compared based on their
// weight.

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}
impl Eq for Member {}

impl PartialOrd for Member {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.weight.cmp(&other.weight))
    }
}

impl Ord for Member {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}

impl Hash for Member {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
    }
}
