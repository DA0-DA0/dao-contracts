use cosmwasm_schema::cw_serde;

use crate::nft_stake::NftStakeChangedHookMsg;
use crate::proposal::ProposalHookMsg;
use crate::stake::StakeChangedHookMsg;
use crate::vote::VoteHookMsg;

/// An enum representing all possible DAO hooks.
#[cw_serde]
pub enum DaoHooks {
    /// Called when NFTs are staked or unstaked.
    NftStakeChangeHook(NftStakeChangedHookMsg),
    /// Called when a proposal status changes.
    ProposalHook(ProposalHookMsg),
    /// Called when tokens are staked or unstaked.
    StakeChangeHook(StakeChangedHookMsg),
    /// Called when a vote is cast.
    VoteHook(VoteHookMsg),
}
