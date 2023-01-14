use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Uint128};
use cw_utils::Expiration;

use crate::{
    m::{Stats, M},
    vote::Vote,
};

/// Stores the state of a ranked choice election by wrapping a `M`
/// matrix and maintaining:
///
/// LM[x][y] = |x > y| - |y > x|
///
/// Or in english "the number of times x has beaten y" minus "the
/// number of times y has beaten x". This construction provides that
/// if a column holds all positive, non-zero values then the
/// corresponding candidate is the Condorcet winner. A Condorcet
/// winner is undisputed if it's smallest margin of victory is larger
/// than the outstanding voting power.
#[cw_serde]
pub struct Tally {
    m: M,

    /// When this tally will stop accepting votes.
    pub expiration: Expiration,
    /// The block height that this tally began at.
    pub start_height: u64,
    /// Amount of voting power that has yet to vote in this tally.
    pub power_outstanding: Uint128,
    /// The current winner. Always up to date and updated on vote.
    pub winner: Winner,
}

#[cw_serde]
#[derive(Copy)]
pub enum Winner {
    Never,
    None,
    Some(u32),
    Undisputed(u32),
}

impl Tally {
    pub fn new(
        candidates: u32,
        total_power: Uint128,
        start_height: u64,
        expiration: Expiration,
    ) -> Self {
        let mut tally = Self {
            m: M::new(candidates),
            power_outstanding: total_power,
            winner: Winner::None,
            start_height,
            expiration,
        };
        // compute even though this will always be Winner::None so
        // that creating a tally has the same compute cost of adding a
        // vote which is needed so that gas(proposal_creation) >=
        // gas(vote).
        tally.winner = tally.winner();
        tally
    }

    pub fn candidates(&self) -> u32 {
        self.m.n
    }

    pub fn expired(&self, block: &BlockInfo) -> bool {
        self.expiration.is_expired(block)
    }

    /// Records a vote in the tally. The tally must not be expired.
    ///
    ///  - `vote` a list of candidates sorted in order from most to
    ///    least favored
    ///  - `power` the voting power of the voter
    ///
    /// Invariants:
    ///
    /// - Voter has not already voted.
    /// - Tally is not expired.
    pub fn add_vote(&mut self, vote: Vote, power: Uint128) {
        for (index, preference) in vote.iter().enumerate() {
            // an interesting property of the symetry of M is that in
            // recording all the defeats, we also record all of the
            // victories.
            for defeat in 0..index {
                self.m.decrement((*preference, vote[defeat]), power)
            }
        }
        self.power_outstanding -= power;
        self.winner = self.winner();
    }

    fn winner(&self) -> Winner {
        match self.m.stats(self.power_outstanding) {
            Stats::PositiveColumn { col, min_margin } => {
                if min_margin > self.power_outstanding {
                    Winner::Undisputed(col)
                } else {
                    Winner::Some(col)
                }
            }
            Stats::NoPositiveColumn {
                no_winnable_columns,
            } => {
                if no_winnable_columns {
                    Winner::Never
                } else {
                    Winner::None
                }
            }
        }
    }
}
