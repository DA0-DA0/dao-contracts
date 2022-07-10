use cosmwasm_std::{CosmosMsg, Empty};
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_core_macros::govmod_query;
use voting::{deposit::DepositInfo, threshold::Threshold, voting::Vote};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The threshold a proposal must reach to complete.
    pub threshold: Threshold,
    /// The default maximum amount of time a proposal may be voted on
    /// before expiring.
    pub max_voting_period: Duration,
    /// The minimum amount of time a proposal must be open before
    /// passing. A proposal may fail before this amount of time has
    /// elapsed, but it will not pass. This can be useful for
    /// preventing governance attacks wherein an attacker aquires a
    /// large number of tokens and forces a proposal through.
    pub min_voting_period: Option<Duration>,
    /// If set to true only members may execute passed
    /// proposals. Otherwise, any address may execute a passed
    /// proposal.
    pub only_members_execute: bool,
    /// Allows changing votes before the proposal expires. If this is
    /// enabled proposals will not be able to complete early as final
    /// vote information is not known until the time of proposal
    /// expiration.
    pub allow_revoting: bool,
    /// Information about the deposit required to create a
    /// proposal. None if there is no deposit requirement, Some
    /// otherwise.
    pub deposit_info: Option<DepositInfo>,
}

/// Information about the token to use for proposal deposits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositToken {
    /// Use a specific token address as the deposit token.
    Token { address: String },
    /// Use the token address of the associated DAO's voting
    /// module. NOTE: in order to use the token address of the voting
    /// module the voting module must (1) use a cw20 token and (2)
    /// implement the `TokenContract {}` query type defined by
    /// `cw_core_macros::token_query`. Failing to implement that
    /// and using this option will cause instantiation to fail.
    VotingModuleToken {},
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
        /// The messages that should be executed in response to this
        /// proposal passing.
        msgs: Vec<CosmosMsg<Empty>>,
    },
    /// Votes on a proposal. Voting power is determined by the DAO's
    /// voting power module.
    Vote {
        /// The ID of the proposal to vote on.
        proposal_id: u64,
        /// The senders position on the proposal.
        vote: Vote,
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
        /// The new proposal passing threshold. This will only apply
        /// to proposals created after the config update.
        threshold: Threshold,
        /// The default maximum amount of time a proposal may be voted
        /// on before expiring. This will only apply to proposals
        /// created after the config update.
        max_voting_period: Duration,
        /// The minimum amount of time a proposal must be open before
        /// passing. A proposal may fail before this amount of time has
        /// elapsed, but it will not pass. This can be useful for
        /// preventing governance attacks wherein an attacker aquires a
        /// large number of tokens and forces a proposal through.
        min_voting_period: Option<Duration>,
        /// If set to true only members may execute passed
        /// proposals. Otherwise, any address may execute a passed
        /// proposal. Applies to all outstanding and future proposals.
        only_members_execute: bool,
        /// Allows changing votes before the proposal expires. If this is
        /// enabled proposals will not be able to complete early as final
        /// vote information is not known until the time of proposal
        /// expiration.
        allow_revoting: bool,
        /// The address if tge DAO that this governance module is
        /// associated with.
        dao: String,
        /// Information about the deposit required to make a
        /// proposal. None if no deposit, Some otherwise.
        deposit_info: Option<DepositInfo>,
    },
    /// Adds an address as a consumer of proposal hooks. Consumers of
    /// proposal hooks have hook messages executed on them whenever
    /// the status of a proposal changes or a proposal is created. If
    /// a consumer contract errors when handling a hook message it
    /// will be removed from the list of consumers.
    AddProposalHook { address: String },
    /// Removes a consumer of proposal hooks.
    RemoveProposalHook { address: String },
    /// Adds an address as a consumer of vote hooks. Consumers of vote
    /// hooks have hook messages executed on them whenever the a vote
    /// is cast. If a consumer contract errors when handling a hook
    /// message it will be removed from the list of consumers.
    AddVoteHook { address: String },
    /// Removed a consumer of vote hooks.
    RemoveVoteHook { address: String },
}

#[govmod_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Gets the governance module's config. Returns `state::Config`.
    Config {},
    /// Gets information about a proposal. Returns
    /// `proposals::Proposal`.
    Proposal { proposal_id: u64 },
    /// Lists all the proposals that have been cast in this
    /// module. Returns `query::ProposalListResponse`.
    ListProposals {
        /// The proposal ID to start listing proposals after. For
        /// example, if this is set to 2 proposals with IDs 3 and
        /// higher will be returned.
        start_after: Option<u64>,
        /// The maximum number of proposals to return as part of this
        /// query. If no limit is set a max of 30 proposals will be
        /// returned.
        limit: Option<u64>,
    },
    /// Lists all of the proposals that have been cast in this module
    /// in decending order of proposal ID. Returns
    /// `query::ProposalListResponse`.
    ReverseProposals {
        /// The proposal ID to start listing proposals before. For
        /// example, if this is set to 6 proposals with IDs 5 and
        /// lower will be returned.
        start_before: Option<u64>,
        /// The maximum number of proposals to return as part of this
        /// query. If no limit is set a max of 30 proposals will be
        /// returned.
        limit: Option<u64>,
    },
    /// Returns the number of proposals that have been created in this
    /// module.
    ProposalCount {},
    /// Returns a voters position on a propsal. Returns
    /// `query::VoteResponse`.
    GetVote { proposal_id: u64, voter: String },
    /// Lists all of the votes that have been cast on a
    /// proposal. Returns `VoteListResponse`.
    ListVotes {
        /// The proposal to list the votes of.
        proposal_id: u64,
        /// The voter to start listing votes after. Ordering is done
        /// alphabetically.
        start_after: Option<String>,
        /// The maximum number of votes to return in response to this
        /// query. If no limit is specified a max of 30 are returned.
        limit: Option<u64>,
    },
    /// Lists all of the consumers of proposal hooks for this module.
    ProposalHooks {},
    /// Lists all of the consumers of vote hooks for this
    /// module. Returns indexable_hooks::HooksResponse.
    VoteHooks {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
