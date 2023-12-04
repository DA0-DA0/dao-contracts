use cosmwasm_schema::cw_serde;
use cw4::MemberChangedHookMsg;

use crate::nft_stake::NftStakeChangedHookMsg;
use crate::proposal::{PreProposeHookMsg, ProposalHookMsg};
use crate::stake::StakeChangedHookMsg;
use crate::vote::VoteHookMsg;

/// An enum representing all possible DAO hooks.
#[cw_serde]
pub enum DaoHooks {
    /// Called when a member is added or removed
    /// to a cw4-groups or cw721-roles contract.
    MemberChangedHook(MemberChangedHookMsg),
    /// Called when NFTs are staked or unstaked.
    NftStakeChangeHook(NftStakeChangedHookMsg),
    /// Pre-propose hooks
    PreProposeHook(PreProposeHookMsg),
    /// Called when a proposal status changes.
    ProposalHook(ProposalHookMsg),
    /// Called when tokens are staked or unstaked.
    StakeChangeHook(StakeChangedHookMsg),
    /// Called when a vote is cast.
    VoteHook(VoteHookMsg),
}
