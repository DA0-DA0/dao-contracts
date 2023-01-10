use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

use crate::{m::LM, vote::Vote};

#[cw_serde]
pub struct Tally(LM);

impl Tally {
    pub fn new(candidates: u32) -> Self {
        Self(LM::new(candidates as usize))
    }

    /// arguments:
    ///  - `vote` a list of candidates sorted in order from most to
    ///    least favored
    ///  - `power` the voting power of the voter
    ///
    /// invariants:
    ///   - `vote` contains no duplicate values
    ///   - `vote` contains one entry for each candidate
    pub fn add_vote(&mut self, vote: Vote, power: Uint128) {
        for (index, preference) in vote.iter().enumerate() {
            // an interesting property of the symetry of M is that in
            // recording all the defeats, we also record all of the
            // victories.
            for defeat in 0..index {
                self.0
                    .decrement((*preference as usize, vote[defeat] as usize), power)
            }
        }
    }

    // winner(votes_possible) -> (winner, undisputed)
    pub fn winner(&self, votes_cast: Uint128, votes_possible: Uint128) -> Option<(u32, bool)> {
        self.0
            .positive_row_and_margin()
            .map(|(row, margin)| (row as u32, margin > votes_possible - votes_cast))
    }
}
