use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_denom::CheckedDenom;
use cw_storage_plus::{SnapshotItem, Strategy};

/// Incentives for passing successful proposals
#[cw_serde]
pub struct ProposalIncentives {
    /// The rewards to pay out per successful proposal.
    pub rewards_per_proposal: Uint128,
    pub denom: CheckedDenom,
}

/// Holds ProposalIncentives state
pub const PROPOSAL_INCENTIVES: SnapshotItem<ProposalIncentives> = SnapshotItem::new(
    "proposal_incentives",
    "proposal_incentives__check",
    "proposal_incentives__change",
    Strategy::EveryBlock,
);
