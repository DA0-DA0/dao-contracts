use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

/// The address of the DAO this contract serves
pub const DAO: Item<Addr> = Item::new("dao");

/// Incentives for passing successful proposals
#[cw_serde]
pub struct ProposalIncentives {
    /// The rewards to pay out per successful proposal.
    pub rewards_per_proposal: Coin,
}

/// Holds ProposalIncentives state
pub const PROPOSAL_INCENTIVES: Item<ProposalIncentives> = Item::new("proposal_incentives");
