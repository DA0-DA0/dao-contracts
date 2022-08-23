use crate::{
    proposal::MultipleChoiceProposal,
    state::{Config, VoteInfo},
};
use cosmwasm_std::Uint128;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::voting::MultipleChoiceVote;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}

/// Information about a proposal returned by proposal queries.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalResponse {
    pub id: u64,
    pub proposal: MultipleChoiceProposal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoteResponse {
    pub vote: Option<VoteInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoteListResponse {
    pub votes: Vec<VoteInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoterResponse {
    pub weight: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ConfigResponse {
    pub config: Config,
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
    Voted(MultipleChoiceVote),
    NotVoted {},
    AnyVote {},
}
