use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use cw4::MemberChangedHookMsg;
use cw_ownable::cw_ownable_execute;
use cw_utils::Duration;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};
use dao_interface::voting::InfoResponse;

pub use cw_ownable::Ownership;

use crate::state::Delegation;

#[cw_serde]
pub struct InstantiateMsg {
    /// The DAO. If not provided, the instantiator is used.
    pub dao: Option<String>,
    /// the maximum percent of voting power that a single delegate can wield.
    /// they can be delegated any amount of voting power—this cap is only
    /// applied when casting votes.
    pub vp_cap_percent: Option<Decimal>,
    /// the duration a delegation is valid for, after which it must be renewed
    /// by the delegator.
    pub delegation_validity: Option<Duration>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Called when a member is added or removed
    /// to a cw4-groups or cw721-roles contract.
    MemberChangedHook(MemberChangedHookMsg),
    /// Called when NFTs are staked or unstaked.
    NftStakeChangeHook(NftStakeChangedHookMsg),
    /// Called when tokens are staked or unstaked.
    StakeChangeHook(StakeChangedHookMsg),
    /// updates the configuration of the delegation system
    UpdateConfig {
        /// the maximum percent of voting power that a single delegate can
        /// wield. they can be delegated any amount of voting power—this cap is
        /// only applied when casting votes.
        vp_cap_percent: Option<Decimal>,
        /// the duration a delegation is valid for, after which it must be
        /// renewed by the delegator.
        delegation_validity: Option<Duration>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    /// Returns information about the ownership of this contract.
    #[returns(Ownership<Addr>)]
    Ownership {},
    /// Returns the delegations by a delegator.
    #[returns(DelegationsResponse)]
    DelegatorDelegations {
        delegator: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns the delegations to a delegate.
    #[returns(DelegationsResponse)]
    DelegateDelegations {
        delegate: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct DelegationsResponse {
    pub delegations: Vec<Delegation>,
}

#[cw_serde]
pub struct MigrateMsg {}
