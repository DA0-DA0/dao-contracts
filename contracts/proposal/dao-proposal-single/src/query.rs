use crate::proposal::SingleChoiceProposal;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use dao_voting::voting::Vote;

/// Information about a proposal returned by proposal queries.
#[cw_serde]
pub struct ProposalResponse {
    /// The ID of the proposal being returned.
    pub id: u64,
    pub proposal: SingleChoiceProposal,
}

/// Information about a vote that was cast.
#[cw_serde]
pub struct VoteInfo {
    /// The address that voted.
    pub voter: Addr,
    /// Position on the vote.
    pub vote: Vote,
    /// The voting power behind the vote.
    pub power: Uint128,
    /// Address-specified rationale for the vote.
    pub rationale: Option<String>,
}

/// Information about a vote.
#[cw_serde]
pub struct VoteResponse {
    /// None if no such vote, Some otherwise.
    pub vote: Option<VoteInfo>,
}

/// Information about the votes for a proposal.
#[cw_serde]
pub struct VoteListResponse {
    pub votes: Vec<VoteInfo>,
}

/// A list of proposals returned by `ListProposals` and
/// `ReverseProposals`.
#[cw_serde]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}
