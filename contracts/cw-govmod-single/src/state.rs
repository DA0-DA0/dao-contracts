use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    proposal::{Proposal, Vote},
    threshold::Threshold,
};

/// The governance module's configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The threshold a proposal must reach to complete.
    pub threshold: Threshold,
    /// The default maximum amount of time a proposal may be voted on
    /// before expiring.
    pub max_voting_period: Duration,
    /// If set to true only members may execute passed
    /// proposals. Otherwise, any address may execute a passed
    /// proposal.
    pub only_members_execute: bool,
    /// The address of the DAO that this governance module is
    /// associated with.
    pub dao: Addr,
}

/// A vote cast for a proposal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ballot {
    /// The amount of voting power behind the vote.
    pub power: Uint128,
    /// The position.
    pub vote: Vote,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");
pub const BALLOTS: Map<(u64, Addr), Ballot> = Map::new("ballots");
