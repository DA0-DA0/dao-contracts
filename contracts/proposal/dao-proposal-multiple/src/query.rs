use crate::{proposal::MultipleChoiceProposal, state::Config};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use dao_voting::multiple_choice::MultipleChoiceVote;

#[cw_serde]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}

/// Information about a proposal returned by proposal queries.
#[cw_serde]
pub struct ProposalResponse {
    pub id: u64,
    pub proposal: MultipleChoiceProposal,
}

/// Information about a vote that was cast.
#[cw_serde]
pub struct VoteInfo {
    /// The address that voted.
    pub voter: Addr,
    /// Position on the vote.
    pub vote: MultipleChoiceVote,
    /// The voting power behind the vote.
    pub power: Uint128,
    /// The rationale behind the vote.
    pub rationale: Option<String>,
}

#[cw_serde]
pub struct VoteResponse {
    pub vote: Option<VoteInfo>,
}

#[cw_serde]
pub struct VoteListResponse {
    pub votes: Vec<VoteInfo>,
}

#[cw_serde]
pub struct VoterResponse {
    pub weight: Option<Uint128>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}
