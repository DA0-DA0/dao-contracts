use std::convert::TryInto;

use cosmwasm_std::{Addr, BlockInfo, CosmosMsg, Empty, StdError, StdResult, Storage, Uint128};
use cw3::Status;
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{helpers::votes_needed, msg::Threshold};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    pub proposal_deposit: Uint128,
    pub refund_failed_proposals: Option<bool>,
    pub gov_token_address: Addr,
    pub parent_dao_contract_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub choices: Vec<String>,
    pub proposer: Addr,
    pub start_height: u64,
    pub expires: Expiration,
    pub msgs: Vec<Vec<CosmosMsg<Empty>>>,
    pub status: Status,
    // Selected choice is index in choices
    pub selected_choice: Option<usize>,
    /// Pass requirements
    pub threshold: Threshold,
    /// The total weight when the proposal started (used to calculate percentages)
    pub total_weight: Uint128,
    /// summary of existing votes
    pub votes: Votes,
    /// Amount of the native governance token required for voting
    pub deposit: Uint128,
}

impl Proposal {
    /// current_status is non-mutable and returns what the status should be.
    /// (designed for queries)
    pub fn current_status(&mut self, block: &BlockInfo) -> Status {
        let mut status = self.status;

        // if open, check if voting is passed or timed out
        if status == Status::Open && self.is_passed(block) {
            status = Status::Passed;
            self.selected_choice = Some(
                self.votes
                    .vote_counts
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.cmp(b))
                    .map(|(idx, _)| idx)
                    .unwrap(),
            );
        }
        if status == Status::Open && self.expires.is_expired(block) {
            status = Status::Rejected;
        }

        status
    }

    /// update_status sets the status of the proposal to current_status.
    /// (designed for handler logic)
    pub fn update_status(&mut self, block: &BlockInfo) {
        self.status = self.current_status(block);
    }

    pub fn is_passed(&self, block: &BlockInfo) -> bool {
        match self.threshold {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => {
                for c in &self.votes.vote_counts {
                    if *c >= votes_needed(self.total_weight, percentage_needed) {
                        return true;
                    }
                }
                return false;
            }
            Threshold::ThresholdQuorum { threshold, quorum } => {
                // we always require the quorum
                if self.votes.total() < votes_needed(self.total_weight, quorum) {
                    return false;
                }
                for c in &self.votes.vote_counts {
                    if self.expires.is_expired(block) {
                        // If expired, we compare vote_count against the total number of votes.
                        if *c >= votes_needed(self.votes.total(), threshold) {
                            return true;
                        }
                    } else {
                        // If not expired, we must assume all non-votes will be cast against
                        // vote_count
                        if *c >= votes_needed(self.total_weight, threshold) {
                            return true;
                        }
                    }
                }
                return false;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
pub struct Vote {
    // A vote indicates which option (index in vote_counts) the user has selected.
    pub option: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Votes {
    // Vote counts is a vector of integers indicating the vote weight for each option.
    pub vote_counts: Vec<Uint128>,
}

impl Votes {
    /// sum of all votes
    pub fn total(&self) -> Uint128 {
        self.vote_counts.iter().sum()
    }

    pub fn add_vote(&mut self, vote: Vote, weight: Uint128) {
        self.vote_counts[vote.option] += weight;
    }
}

/// Returns the vote (opinion as well as weight counted) as well as
/// the address of the voter who submitted it
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteInfo {
    pub voter: String,
    pub vote: Vote,
    pub weight: Uint128,
}

pub fn parse_id(data: &[u8]) -> StdResult<u64> {
    match data[0..8].try_into() {
        Ok(bytes) => Ok(u64::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 8 byte expected.",
        )),
    }
}

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ballot {
    pub weight: Uint128,
    pub vote: Vote,
}

pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
    PROPOSAL_COUNT.save(store, &id)?;
    Ok(id)
}

pub const BALLOTS: Map<(u64, &Addr), Ballot> = Map::new("multiple_choice_votes");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("multiple_choice_proposals");

pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
