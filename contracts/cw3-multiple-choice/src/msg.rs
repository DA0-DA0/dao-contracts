use cosmwasm_std::{CosmosMsg, Decimal, Empty, Uint128};
use cw_utils::{Duration, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{query::ThresholdResponse, state::Vote, ContractError};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    /// Voting params configuration
    pub threshold: Threshold,
    /// The amount of time a proposal can be voted on before expiring
    pub max_voting_period: Duration,
    /// Deposit required to make a proposal
    pub proposal_deposit_amount: Uint128,
    /// Refund a proposal if it is rejected
    pub refund_failed_proposals: Option<bool>,
    /// The existing governance token address
    pub gov_token_address: String,
    /// The parent dao contract address
    pub parent_dao_contract_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Makes a new proposal
    Propose(ProposeMsg),
    /// Vote on an open proposal
    Vote(VoteMsg),
    /// Execute a passed proposal
    Execute { proposal_id: u64 },
    /// Close a failed proposal
    Close { proposal_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return ThresholdResponse
    Threshold {},
    /// Returns ProposalResponse
    Proposal {
        proposal_id: u64,
    },
    /// Returns ProposalListResponse
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns ProposalListResponseb
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns the number of proposals in the DAO (u64)
    ProposalCount {},
    // Returns config
    GetConfig {},
    /// Returns VoteResponse
    Vote {
        proposal_id: u64,
        voter: String,
    },
    /// Returns VoteListResponse
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns information about current tallys for a
    /// proposal. Returns type `VoteTallyResponse`.
    Tally {
        proposal_id: u64,
    },
    /// Returns VoterInfo
    Voter {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VoteMsg {
    pub proposal_id: u64,
    pub vote: Vote,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposeMsg {
    pub title: String,
    pub description: String,
    pub choices: Vec<String>,
    pub msgs: Vec<Vec<CosmosMsg<Empty>>>,
    // note: we ignore API-spec'd earliest if passed, always opens immediately
    pub latest: Option<Expiration>,
}

/// This defines the different ways tallies can happen.
///
/// The total_weight used for calculating success as well as the weights of each
/// individual voter used in tallying should be snapshotted at the beginning of
/// the block at which the proposal starts (this is likely the responsibility of a
/// correct cw4 implementation).
/// See also `ThresholdResponse` in the cw3 spec.
/// This is similar to thresholds for 'yes' votes in single choice voting, but in this case in order for any
/// of the multiple choice options to pass, it must have threshold votes in its favor.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Threshold {
    /// Declares a percentage of the total weight that must cast a vote for any given option in order for
    /// a proposal to be executed.
    AbsolutePercentage { percentage: Decimal },

    /// Declares a `quorum` of the total votes that must participate in the election in order
    /// for the vote to be considered at all. Within that quorum, threshold must vote in favor of
    /// any given option.
    ThresholdQuorum { threshold: Decimal, quorum: Decimal },
}

impl Threshold {
    /// returns error if this is an unreachable value,
    /// given a total weight of all members in the group
    pub fn validate(&self) -> Result<(), ContractError> {
        match self {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => valid_percentage(percentage_needed),
            Threshold::ThresholdQuorum {
                threshold,
                quorum: quroum,
            } => {
                valid_percentage(threshold)?;
                valid_percentage(quroum)
            }
        }
    }

    /// Creates a response from the saved data, just missing the total_weight info
    pub fn to_response(&self, total_weight: Uint128) -> ThresholdResponse {
        match self.clone() {
            Threshold::AbsolutePercentage { percentage } => ThresholdResponse::AbsolutePercentage {
                percentage,
                total_weight,
            },
            Threshold::ThresholdQuorum { threshold, quorum } => {
                ThresholdResponse::ThresholdQuorum {
                    threshold,
                    quorum,
                    total_weight,
                }
            }
        }
    }
}

/// Asserts that the 0.0 < percent <= 1.0
fn valid_percentage(percent: &Decimal) -> Result<(), ContractError> {
    if percent.is_zero() {
        Err(ContractError::ZeroThreshold {})
    } else if *percent > Decimal::one() {
        Err(ContractError::UnreachableThreshold {})
    } else {
        Ok(())
    }
}
