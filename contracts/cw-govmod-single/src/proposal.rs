use std::convert::TryInto;

use cosmwasm_std::{Addr, BlockInfo, CosmosMsg, Decimal, Empty, Uint128, Uint256};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::threshold::Threshold;

// We multiply by this when calculating needed_votes in order to round
// up properly.
const PRECISION_FACTOR: u128 = 10u128.pow(9);

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Status {
    Open,
    Rejected,
    Passed,
    Executed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Votes {
    pub yes: Uint128,
    pub no: Uint128,
    pub abstain: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,

    pub start_height: u64,
    pub expiration: Expiration,

    pub threshold: Threshold,
    pub total_power: Uint128,

    pub msgs: Vec<CosmosMsg<Empty>>,

    pub status: Status,
    pub votes: Votes,
}

impl Votes {
    pub fn zero() -> Self {
        Self {
            yes: Uint128::zero(),
            no: Uint128::zero(),
            abstain: Uint128::zero(),
        }
    }
}

/// Computes the number of votes needed for a proposal to pass. This
/// must round up. For example, with a 50% passing percentage and 15
/// total votes 8 votes are required, not 7.
fn votes_needed(total_power: Uint128, passing_percentage: Decimal) -> Uint128 {
    // Voting power is counted with a Uint128. In order to avoid an
    // overflow while multiplying by the percision factor we need to
    // do a full mul which results in a Uint256.
    let total_power = total_power.full_mul(PRECISION_FACTOR);
    // Multiplication of a Uint256 and a Decimal is not implemented so
    // we first multiply by the decimal's raw form and then divide by
    // the number of decimal places in the decimal. This is the same
    // thing that happens under the hood when a Uint128 is multiplied
    // by Decimal.
    //
    // This multiplication will round down but because we have
    // multiplied by `PERCISION_FACTOR` above that rounding ought to
    // occur in the 9th decimal place.
    let applied = total_power.multiply_ratio(
        passing_percentage.atomics(),
        passing_percentage.decimal_places(),
    );
    let rounded =
        (applied - Uint256::from(1u128)) / Uint256::from(PRECISION_FACTOR) + Uint256::from(1u128);
    // Truncated should be strictly <= than the largest possible
    // Uint128. Imagine the pathalogical case where the passing
    // threshold is 100% and there are 2^128 total votes. In this case
    // rounded is:
    //
    // Floor[(2^128 * 10^9 - 1) / 10^9 + 1]
    //
    // This number is 2^128 which will fit into a 128 bit
    // integer. Note: if we didn't floor here this would not be the
    // case, happily unsigned integer division does indeed do that.
    let truncated: Uint128 = rounded.try_into().unwrap();
    truncated
}

impl Proposal {
    pub fn current_status(&self, block: &BlockInfo) -> Status {
        todo!()
    }

    pub fn update_status(&mut self, block: &BlockInfo) {
        todo!()
    }

    /// returns true iff this proposal is sure to pass (even before expiration if no future
    /// sequence of possible votes can cause it to fail)
    pub fn is_passed(&self, block: &BlockInfo) -> bool {
        self.does_vote_count_reach_threshold(self.votes.yes, block)
    }

    /// As above for the rejected check, used to check if a proposal is
    /// already rejected.
    pub fn is_rejected(&self, block: &BlockInfo) -> bool {
        self.does_vote_count_reach_threshold(self.votes.no, block)
    }

    /// Helper function to check if a certain vote count has reached threshold.
    /// Only called from is_rejected and is_passed for no and yes votes
    /// Handles the different threshold types accordingly.
    /// This function returns true if and only if vote_count is greater than the threshold which
    /// is calculated.
    /// In the case where we use yes votes, this function will return true if and only if the
    /// proposal will pass.
    /// In the case where we use no votes, this function will return true if and only if the
    /// proposal will be rejected regardless of other votes.
    fn does_vote_count_reach_threshold(&self, vote_count: Uint128, block: &BlockInfo) -> bool {
        todo!()
    }
}
