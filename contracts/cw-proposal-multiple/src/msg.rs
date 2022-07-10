use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::{deposit::DepositInfo, voting::MultipleChoiceVote};

use crate::{state::MultipleChoiceOptions, voting_strategy::VotingStrategy};
use cw_core_macros::govmod_query;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    /// Voting params configuration
    pub voting_strategy: VotingStrategy,
    /// The minimum amount of time a proposal must be open before
    /// passing. A proposal may fail before this amount of time has
    /// elapsed, but it will not pass. This can be useful for
    /// preventing governance attacks wherein an attacker aquires a
    /// large number of tokens and forces a proposal through.
    pub min_voting_period: Option<Duration>,
    /// The amount of time a proposal can be voted on before expiring
    pub max_voting_period: Duration,
    /// If set to true only members may execute passed
    /// proposals. Otherwise, any address may execute a passed
    /// proposal.
    pub only_members_execute: bool,
    /// Information about the deposit required to create a
    /// proposal. None if there is no deposit requirement, Some
    /// otherwise.
    pub deposit_info: Option<DepositInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Creates a proposal in the governance module.
    Propose {
        /// The title of the proposal.
        title: String,
        /// A description of the proposal.
        description: String,
        /// The multiple choices.
        choices: MultipleChoiceOptions,
    },
    /// Votes on a proposal. Voting power is determined by the DAO's
    /// voting power module.
    Vote {
        /// The ID of the proposal to vote on.
        proposal_id: u64,
        /// The senders position on the proposal.
        vote: MultipleChoiceVote,
    },
    /// Causes the messages associated with a passed proposal to be
    /// executed by the DAO.
    Execute {
        /// The ID of the proposal to execute.
        proposal_id: u64,
    },
    /// Closes a proposal that has failed (either not passed or timed
    /// out). If applicable this will cause the proposal deposit
    /// associated wth said proposal to be returned.
    Close {
        /// The ID of the proposal to close.
        proposal_id: u64,
    },
    /// Updates the governance module's config.
    UpdateConfig {
        /// The new proposal voting strategy. This will only apply
        /// to proposals created after the config update.
        voting_strategy: VotingStrategy,
        /// The minimum amount of time a proposal must be open before
        /// passing. A proposal may fail before this amount of time has
        /// elapsed, but it will not pass. This can be useful for
        /// preventing governance attacks wherein an attacker aquires a
        /// large number of tokens and forces a proposal through.
        min_voting_period: Option<Duration>,
        /// The default maximum amount of time a proposal may be voted
        /// on before expiring. This will only apply to proposals
        /// created after the config update.
        max_voting_period: Duration,
        /// If set to true only members may execute passed
        /// proposals. Otherwise, any address may execute a passed
        /// proposal. Applies to all outstanding and future proposals.
        only_members_execute: bool,
        /// The address if tge DAO that this governance module is
        /// associated with.
        dao: String,
        /// Information about the deposit required to make a
        /// proposal. None if no deposit, Some otherwise.
        deposit_info: Option<DepositInfo>,
    },
    AddProposalHook {
        address: String,
    },
    RemoveProposalHook {
        address: String,
    },
    AddVoteHook {
        address: String,
    },
    RemoveVoteHook {
        address: String,
    },
}

#[govmod_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Gets the governance module's config. Returns `state::Config`.
    Config {},
    /// Gets information about a proposal. Returns
    /// `proposals::Proposal`.
    Proposal {
        proposal_id: u64,
    },
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u64>,
    },
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u64>,
    },
    ProposalCount {},
    GetVote {
        proposal_id: u64,
        voter: String,
    },
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u64>,
    },
    ProposalHooks {},
    VoteHooks {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VoteMsg {
    pub proposal_id: u64,
    pub vote: MultipleChoiceVote,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
