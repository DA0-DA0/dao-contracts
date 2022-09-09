//! Here lie all of the types used in v1 proposals, their
//! configurations, and module-level configuration that have changed
//! between DAO DAO v1 and v2. These types are used during migration
//! from v1 to v2 DAOs. Once we start publishing packages on crates.io
//! this will get a lot less verbose.
//!
//! All types here are taken from commit
//! `e531c760a5d057329afd98d62567aaa4dca2c96f`.

use cosmwasm_std::{Addr, CosmosMsg, Empty, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::{status::Status as V2Status, threshold::Threshold, voting::Votes};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug, Copy)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Status {
    Open,
    Rejected,
    Passed,
    Executed,
    Closed,
}

impl From<Status> for V2Status {
    fn from(status: Status) -> Self {
        match status {
            Status::Open => Self::Open,
            Status::Rejected => Self::Rejected,
            Status::Passed => Self::Passed,
            Status::Executed => Self::Executed,
            Status::Closed => Self::Closed,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,
    pub start_height: u64,
    pub min_voting_period: Option<Expiration>,
    pub expiration: Expiration,
    pub threshold: Threshold,
    pub total_power: Uint128,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub status: Status,
    pub votes: Votes,
    pub allow_revoting: bool,
    pub deposit_info: Option<CheckedDepositInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositToken {
    Token { address: String },
    VotingModuleToken {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct DepositInfo {
    pub token: DepositToken,
    pub deposit: Uint128,
    pub refund_failed_proposals: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CheckedDepositInfo {
    pub token: Addr,
    pub deposit: Uint128,
    pub refund_failed_proposals: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    pub min_voting_period: Option<Duration>,
    pub only_members_execute: bool,
    pub allow_revoting: bool,
    pub dao: Addr,
    pub deposit_info: Option<CheckedDepositInfo>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");
