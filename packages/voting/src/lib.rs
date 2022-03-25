use cosmwasm_std::{Decimal, Uint128, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// We multiply by this when calculating needed_votes in order to round
// up properly.
const PRECISION_FACTOR: u128 = 10u128.pow(9);

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Votes {
    pub yes: Uint128,
    pub no: Uint128,
    pub abstain: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Vote {
    /// Marks support for the proposal.
    Yes,
    /// Marks opposition to the proposal.
    No,
    /// Marks participation but does not count towards the ratio of
    /// support / opposed.
    Abstain,
}

/// Information about the number of votes needed to pass a proposal.
#[derive(PartialEq, Clone, Debug)]
pub enum VotesNeeded {
    /// A finite number of votes.
    Finite(Uint128),
    /// An unrechable number of votes. Caused by zero total voting
    /// power.
    Unreachable,
}
/// Computes the number of votes needed for a proposal to pass. This
/// must round up. For example, with a 50% passing percentage and 15
/// total votes 8 votes are required, not 7.
///
/// Note that it is possible, though unlikely, that no number of votes
/// will ever meet the threshold. This happens if the total power is
/// zero. For example, this may happen if all votes are abstain.
pub fn votes_needed(total_power: Uint128, passing_percentage: Decimal) -> Uint128 {
    // Voting power is counted with a Uint128. In order to avoid an
    // overflow while multiplying by the percision factor we need to
    // do a full mul which results in a Uint256.
    //
    // Percision factor here ensures that any rounding down here that
    // happens in the VM happens in the 9th decimal place. This makes
    // it reasonably likely that we successfully round up.
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
        Uint256::from(10u64).pow(passing_percentage.decimal_places()),
    );
    // The maximum possible value for applied occurs if `total_power`
    // is 2^128. Given a percision factor of 10^9 we confirm that the
    // numerator value will not overflow as:
    //
    // 2^128 * 10^9 + 10^9 < 2^256
    //
    // In the interest of being percise, this will not overflow so
    // long as our percision factor is less than `3.4*10^38`. We don't
    // have to worry about this because that value won't even fit into
    // a u128 (the type of PERCISION_FACTOR).
    let rounded = (applied + Uint256::from(PRECISION_FACTOR) - Uint256::from(1u128))
        / Uint256::from(PRECISION_FACTOR);
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
    rounded.try_into().unwrap()
}

/// Determines if a number of votes meets the provided threshold given
/// the total number of votes outstanding.
pub fn votes_meet_threshold(
    votes: Uint128,
    total_power: Uint128,
    passing_percentage: Decimal,
) -> bool {
    votes >= votes_needed(total_power, passing_percentage)
}

impl Votes {
    /// Constructs an zero'd out votes struct.
    pub fn zero() -> Self {
        Self {
            yes: Uint128::zero(),
            no: Uint128::zero(),
            abstain: Uint128::zero(),
        }
    }

    /// Constructs a vote with a specified number of yes votes. Used
    /// for testing.
    #[cfg(test)]
    pub fn with_yes(yes: Uint128) -> Self {
        Self {
            yes,
            no: Uint128::zero(),
            abstain: Uint128::zero(),
        }
    }

    /// Adds a vote to the votes.
    pub fn add_vote(&mut self, vote: Vote, power: Uint128) {
        match vote {
            Vote::Yes => self.yes += power,
            Vote::No => self.no += power,
            Vote::Abstain => self.abstain += power,
        }
    }

    /// Computes the total number of votes cast.
    ///
    /// NOTE: The total number of votes avaliable from a voting module
    /// is a `Uint128`. As it is not possible to vote twice we know
    /// that the sum of votes must be <= 2^128 and can safely return a
    /// `Uint128` from this function. A missbehaving voting power
    /// module may break this invariant.
    pub fn total(&self) -> Uint128 {
        self.yes + self.no + self.abstain
    }
}

impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vote::Yes => write!(f, "yes"),
            Vote::No => write!(f, "no"),
            Vote::Abstain => write!(f, "abstain"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn count_votes() {
        let mut votes = Votes::with_yes(Uint128::new(5));
        votes.add_vote(Vote::No, Uint128::new(10));
        votes.add_vote(Vote::Yes, Uint128::new(30));
        votes.add_vote(Vote::Abstain, Uint128::new(40));

        assert_eq!(votes.total(), Uint128::new(5 + 10 + 30 + 40));
        assert_eq!(votes.yes, Uint128::new(35));
        assert_eq!(votes.no, Uint128::new(10));
        assert_eq!(votes.abstain, Uint128::new(40));
    }

    #[test]
    fn votes_needed_rounds_properly() {
        // round up right below 1
        assert_eq!(
            Uint128::new(1),
            votes_needed(Uint128::new(3), Decimal::permille(333))
        );
        // round up right over 1
        assert_eq!(
            Uint128::new(2),
            votes_needed(Uint128::new(3), Decimal::permille(334))
        );
        assert_eq!(
            Uint128::new(11),
            votes_needed(Uint128::new(30), Decimal::permille(334))
        );

        // exact matches don't round
        assert_eq!(
            Uint128::new(17),
            votes_needed(Uint128::new(34), Decimal::percent(50))
        );
        assert_eq!(
            Uint128::new(12),
            votes_needed(Uint128::new(48), Decimal::percent(25))
        );

        assert_eq!(
            Uint128::new(7),
            votes_needed(Uint128::new(13), Decimal::percent(50))
        );

        assert_eq!(
            Uint128::zero(),
            votes_needed(Uint128::zero(), Decimal::percent(50))
        );

        assert_eq!(
            Uint128::new(1),
            votes_needed(Uint128::new(1), Decimal::percent(1))
        );

        assert_eq!(
            Uint128::new(0),
            votes_needed(Uint128::new(1), Decimal::percent(0))
        );

        assert_eq!(
            Uint128::zero(),
            votes_needed(Uint128::zero(), Decimal::percent(99))
        );
    }

    #[test]
    fn tricky_vote_counts() {
        let threshold = Decimal::percent(50);
        for count in 1..50_000 {
            assert!(votes_meet_threshold(
                Uint128::new(count),
                Uint128::new(count * 2),
                threshold
            ))
        }
        // Zero votes out of zero total power meet any threshold.
        assert!(votes_meet_threshold(
            Uint128::new(0),
            Uint128::new(0),
            threshold
        ));
        assert!(votes_meet_threshold(
            Uint128::zero(),
            Uint128::new(1),
            Decimal::percent(0)
        ));
        assert!(votes_meet_threshold(
            Uint128::zero(),
            Uint128::new(0),
            Decimal::percent(0)
        ))
    }
}
