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
    /// Updates the configuration of the delegation system.
    UpdateConfig {
        /// the maximum percent of voting power that a single delegate can
        /// wield. they can be delegated any amount of voting power—this cap is
        /// only applied when casting votes.
        vp_cap_percent: OptionalUpdate<Decimal>,
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
pub struct MigrateMsg {}
