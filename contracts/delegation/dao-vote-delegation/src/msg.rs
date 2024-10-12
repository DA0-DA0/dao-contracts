use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};
use dao_interface::voting::InfoResponse;

use crate::state::Delegation;

#[cw_serde]
pub struct InstantiateMsg {
    /// The DAO. If not provided, the instantiator is used.
    pub dao: Option<String>,
    /// The authorized voting power changed hook callers.
    pub vp_hook_callers: Option<Vec<String>>,
    /// Whether or not to sync proposal modules initially. If there are too
    /// many, the instantiation will run out of gas, so this should be disabled
    /// and `SyncProposalModules` called manually.
    ///
    /// Defaults to false.
    pub no_sync_proposal_modules: Option<bool>,
    /// the maximum percent of voting power that a single delegate can wield.
    /// they can be delegated any amount of voting power—this cap is only
    /// applied when casting votes.
    pub vp_cap_percent: Option<Decimal>,
    // /// the duration a delegation is valid for, after which it must be renewed
    // /// by the delegator.
    // pub delegation_validity: Option<Duration>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Register as a delegate.
    Register {},
    /// Unregister as a delegate.
    Unregister {},
    /// Create a delegation.
    Delegate {
        /// the delegate to delegate to
        delegate: String,
        /// the percent of voting power to delegate
        percent: Decimal,
    },
    /// Revoke a delegation.
    Undelegate {
        /// the delegate to undelegate from
        delegate: String,
    },
    /// Update the authorized voting power changed hook callers.
    UpdateVotingPowerHookCallers {
        /// the addresses to add.
        add: Option<Vec<String>>,
        /// the addresses to remove.
        remove: Option<Vec<String>>,
    },
    /// Sync the active proposal modules from the DAO. Can be called by anyone.
    SyncProposalModules {
        /// the proposal module to start after, if any. passed through to the
        /// DAO proposal modules query.
        start_after: Option<String>,
        /// the maximum number of proposal modules to return. passed through to
        /// the DAO proposal modules query.
        limit: Option<u32>,
    },
    /// Updates the configuration of the delegation system.
    UpdateConfig {
        /// the maximum percent of voting power that a single delegate can
        /// wield. they can be delegated any amount of voting power—this cap is
        /// only applied when casting votes.
        vp_cap_percent: Option<OptionalUpdate<Decimal>>,
        // /// the duration a delegation is valid for, after which it must be
        // /// renewed by the delegator.
        // delegation_validity: Option<Duration>,
    },
    /// Called when a member is added or removed
    /// to a cw4-groups or cw721-roles contract.
    MemberChangedHook(MemberChangedHookMsg),
    /// Called when NFTs are staked or unstaked.
    NftStakeChangeHook(NftStakeChangedHookMsg),
    /// Called when tokens are staked or unstaked.
    StakeChangeHook(StakeChangedHookMsg),
    /// Called when a vote is cast.
    VoteHook(VoteHookMsg),
}

#[cw_serde]
pub enum OptionalUpdate<T> {
    Set(T),
    Clear,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    #[returns(DelegatesResponse)]
    Delegates {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the delegations by a delegator, optionally at a given height.
    /// Uses the current block height if not provided.
    #[returns(DelegationsResponse)]
    Delegations {
        delegator: String,
        height: Option<u64>,
        offset: Option<u64>,
        limit: Option<u64>,
    },
    /// Returns the VP delegated to a delegate that has not yet been used in
    /// votes cast by delegators in a specific proposal.
    #[returns(Uint128)]
    UnvotedDelegatedVotingPower {
        delegate: String,
        proposal_module: String,
        proposal_id: u64,
        height: u64,
    },
    /// Returns the proposal modules synced from the DAO.
    #[returns(Vec<Addr>)]
    ProposalModules {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the voting power hook callers.
    #[returns(Vec<Addr>)]
    VotingPowerHookCallers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct DelegatesResponse {
    /// The delegates.
    pub delegates: Vec<DelegateResponse>,
}

#[cw_serde]
pub struct DelegateResponse {
    /// The delegate.
    pub delegate: Addr,
    /// The total voting power delegated to the delegate.
    pub power: Uint128,
}

#[cw_serde]
pub struct DelegationsResponse {
    /// The delegations.
    pub delegations: Vec<Delegation>,
    /// The height at which the delegations were loaded.
    pub height: u64,
}

#[cw_serde]
pub struct MigrateMsg {}
