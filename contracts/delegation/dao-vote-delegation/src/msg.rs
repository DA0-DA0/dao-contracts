use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};
use dao_interface::helpers::OptionalUpdate;

// make these types directly available to consumers of this crate
pub use dao_voting::delegation::{
    DelegateResponse, DelegatesResponse, DelegationsResponse, QueryMsg,
};

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
    /// Defaults to `false`.
    pub no_sync_proposal_modules: Option<bool>,
    /// the maximum percent of voting power that a single delegate can wield.
    /// they can be delegated any amount of voting power—this cap is only
    /// applied when casting votes.
    pub vp_cap_percent: Option<Decimal>,
    /// the number of blocks a delegation is valid for, after which it must be
    /// renewed by the delegator. if not set, the delegation will never expire.
    pub delegation_validity_blocks: Option<u64>,
    /// the total number of delegations a member can have. this should be set
    /// based on the max gas allowed in a single block for the given chain.
    ///
    /// this limit is relevant for two reasons:
    ///  1. when voting power is updated for a delegator, we must loop through
    ///     all of their delegates and update their delegated voting power
    ///  2. when a delegator casts a vote on a proposal that overrides their
    ///     delegates' votes, we must loop through all of their delegates and
    ///     update the proposal vote tally accordingly
    ///
    /// in tests on Neutron, with a block max gas of 30M (which is one of the
    /// lowest gas limits on any chain), we found that 50 delegations is a safe
    /// upper bound, so this defaults to 50.
    pub max_delegations: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Register as a delegate.
    Register {},
    /// Unregister as a delegate.
    Unregister {},
    /// Create a delegation or update an existing one.
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
    /// Re-derives an address' delegated voting power from their current
    /// voting power. Can be called by anyone.
    ///
    /// This exists to repair a delegate's total delegated VP after a voting
    /// power change hook for one of their delegators was missed (e.g. due to
    /// a stake/unstake hook failure that is tolerated rather than reverting
    /// the underlying stake change, or any other voting power change that
    /// never reached this contract). Since delegated VP is tracked via
    /// deltas rather than being recomputed from scratch on every query, a
    /// missed hook would otherwise cause permanent drift between a
    /// delegate's total and the sum of what their delegators actually
    /// delegate. This message lets anyone force a resync once the missed
    /// change is noticed.
    ///
    /// If `delegator` is a registered delegate, this behaves like the
    /// delegate branch of the voting power change hook, unregistering them if
    /// they no longer have any voting power. Otherwise, this re-syncs their
    /// contribution to each of their delegates' totals to match their
    /// current voting power.
    Sync {
        /// the address to sync.
        delegator: String,
    },
    /// Updates the configuration of the delegation system.
    UpdateConfig {
        /// the maximum percent of voting power that a single delegate can
        /// wield. they can be delegated any amount of voting power—this cap is
        /// only applied when casting votes.
        vp_cap_percent: OptionalUpdate<Decimal>,
        /// the number of blocks a delegation is valid for, after which it must
        /// be renewed by the delegator. if not set, the delegation will never
        /// expire.
        delegation_validity_blocks: OptionalUpdate<u64>,
        /// the total number of delegations a member can have. this should be
        /// set based on the max gas allowed in a single block for the given
        /// chain.
        ///
        /// this limit is relevant for two reasons:
        ///  1. when voting power is updated for a delegator, we must loop
        ///     through all of their delegates and update their delegated voting
        ///     power
        ///  2. when a delegator casts a vote on a proposal that overrides their
        ///     delegates' votes, we must loop through all of their delegates
        ///     and update the proposal vote tally accordingly
        max_delegations: Option<u64>,
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
pub struct MigrateMsg {}
