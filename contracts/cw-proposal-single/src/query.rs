use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use voting::voting::Vote;

use crate::proposal::SingleChoiceProposal;

/// Information about a proposal returned by proposal queries.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalResponse {
    /// The ID of the proposal being returned.
    pub id: u64,
    pub proposal: SingleChoiceProposal,
}

/// Information about a vote that was cast.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoteInfo {
    /// The address that voted.
    pub voter: Addr,
    /// Position on the vote.
    pub vote: Vote,
    /// The voting power behind the vote.
    pub power: Uint128,
}

/// Information about a vote.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoteResponse {
    /// None if no such vote, Some otherwise.
    pub vote: Option<VoteInfo>,
}

/// Information about the votes for a proposal.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoteListResponse {
    pub votes: Vec<VoteInfo>,
}

/// A list of proposals returned by `ListProposals` and
/// `ReverseProposals`.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct FilterListProposalsResponse {
    pub proposals: Vec<ProposalResponse>,
    /// Last checked `proposal_id`.
    /// For example, if contract have 3 proposals and `FilterListProposals`
    /// returned only first two - it will be 3(if limit > 2).
    pub last_proposal_id: u64,
}

/// Helper struct for [`crate::msg::QueryMsg::FilterListProposals`]
/// Letting users to specify what types of wallet votes they are looking for
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "lowercase")]
pub enum WalletVote {
    Voted { vote: Vote },
    DidNotVote {},
    AnyVote {},
}
