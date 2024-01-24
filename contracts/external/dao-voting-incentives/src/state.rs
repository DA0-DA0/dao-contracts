use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{Item, Map};

/// The address of the DAO this contract serves
pub const DAO: Item<Addr> = Item::new("dao");

/// Incentives for voting
#[cw_serde]
pub struct VotingIncentives {
    /// Epoch duration in seconds. Used for reward calculation.
    pub epoch_duration: Timestamp,
    /// The rewards to pay out per epoch.
    pub rewards_per_epoch: Coin,
}

/// Holds VotingIncentives state
pub const VOTING_INCENTIVES: Item<VotingIncentives> = Item::new("voting_incentives");

/// The current epoch
pub const EPOCH: Item<u64> = Item::new("epoch");

/// A map of addresses to their last claimed epoch
pub const LAST_CLAIMED_EPOCHS: Map<Addr, u64> = Map::new("last_claimed_epoch");

/// A map of epochs to prop count
pub const EPOCH_PROPOSAL_COUNT: Map<u64, u64> = Map::new("epoch_proposal_count");

/// A map of epochs to total vote count
pub const EPOCH_TOTAL_VOTE_COUNT: Map<u64, u64> = Map::new("epoch_total_vote_count");

/// A map of user addresses + epoch to vote count
pub const USER_EPOCH_VOTE_COUNT: Map<(Addr, u64), u64> = Map::new("user_epoch_vote_count");
