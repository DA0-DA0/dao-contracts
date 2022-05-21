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

pub enum VoteCmp {
    Greater,
    Geq,
}

/// Compares `votes` with `total_power * passing_percentage`. The
/// comparason function used depends on the `VoteCmp` variation
/// selected.
///
/// !!NOTE!! THIS FUNCTION DOES NOT ROUND UP.
///
/// For example, the following assertion will succede:
///
/// ```rust
/// use voting::{compare_vote_count, VoteCmp};
/// use cosmwasm_std::{Uint128, Decimal};
/// fn test() {
///     assert!(compare_vote_count(
///         Uint128::new(7),
///         VoteCmp::Greater,
///         Uint128::new(13),
///         Decimal::from_ratio(7u64, 13u64)
///     ));
/// }
/// ```
///
/// This is because `7 * (7/13)` is `6.999...` after rounding. You
/// MUST ensure this is the behavior you want when calling this
/// function.
///
/// For our current purposes this is OK as the only place we use the
/// `Greater` comparason is when looking to see if no votes have
/// reached `> (1 - threshold)` and thus made the proposal
/// unpassable. As threshold will be a rounded down version of the
/// infinite percision real version `1 - threshold` will actually be a
/// higher magnitured than the real one meaning that we won't ever be
/// in the position of simeltaniously reporting a proposal as both
/// rejected and passed.
///
pub fn compare_vote_count(
    votes: Uint128,
    cmp: VoteCmp,
    total_power: Uint128,
    passing_percentage: Decimal,
) -> bool {
    let votes = votes.full_mul(PRECISION_FACTOR);
    let total_power = total_power.full_mul(PRECISION_FACTOR);
    let threshold = total_power.multiply_ratio(
        passing_percentage.atomics(),
        Uint256::from(10u64).pow(passing_percentage.decimal_places()),
    );
    match cmp {
        VoteCmp::Greater => votes > threshold,
        VoteCmp::Geq => votes >= threshold,
    }
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

    /// Removes a vote from the votes. The vote being removed must
    /// have been previously added or this method will cause an
    /// overflow.
    pub fn remove_vote(&mut self, vote: Vote, power: Uint128) {
        match vote {
            Vote::Yes => self.yes -= power,
            Vote::No => self.no -= power,
            Vote::Abstain => self.abstain -= power,
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
    fn vote_comparasons() {
        assert!(!compare_vote_count(
            Uint128::new(7),
            VoteCmp::Geq,
            Uint128::new(15),
            Decimal::percent(50)
        ));
        assert!(!compare_vote_count(
            Uint128::new(7),
            VoteCmp::Greater,
            Uint128::new(15),
            Decimal::percent(50)
        ));

        assert!(compare_vote_count(
            Uint128::new(7),
            VoteCmp::Geq,
            Uint128::new(14),
            Decimal::percent(50)
        ));
        assert!(!compare_vote_count(
            Uint128::new(7),
            VoteCmp::Greater,
            Uint128::new(14),
            Decimal::percent(50)
        ));

        assert!(compare_vote_count(
            Uint128::new(7),
            VoteCmp::Geq,
            Uint128::new(13),
            Decimal::from_ratio(7u64, 13u64)
        ));

        assert!(!compare_vote_count(
            Uint128::new(6),
            VoteCmp::Greater,
            Uint128::new(13),
            Decimal::one() - Decimal::from_ratio(7u64, 13u64)
        ));
        assert!(compare_vote_count(
            Uint128::new(7),
            VoteCmp::Greater,
            Uint128::new(13),
            Decimal::from_ratio(7u64, 13u64)
        ));

        assert!(!compare_vote_count(
            Uint128::new(4),
            VoteCmp::Geq,
            Uint128::new(9),
            Decimal::percent(50)
        ))
    }

    #[test]
    fn more_votes_tests() {
        assert!(compare_vote_count(
            Uint128::new(1),
            VoteCmp::Geq,
            Uint128::new(3),
            Decimal::permille(333)
        ));

        assert!(!compare_vote_count(
            Uint128::new(1),
            VoteCmp::Geq,
            Uint128::new(3),
            Decimal::permille(334)
        ));
        assert!(compare_vote_count(
            Uint128::new(2),
            VoteCmp::Geq,
            Uint128::new(3),
            Decimal::permille(334)
        ));

        assert!(compare_vote_count(
            Uint128::new(11),
            VoteCmp::Geq,
            Uint128::new(30),
            Decimal::permille(333)
        ));

        assert!(compare_vote_count(
            Uint128::new(15),
            VoteCmp::Geq,
            Uint128::new(30),
            Decimal::permille(500)
        ));
        assert!(!compare_vote_count(
            Uint128::new(15),
            VoteCmp::Greater,
            Uint128::new(30),
            Decimal::permille(500)
        ));

        assert!(compare_vote_count(
            Uint128::new(0),
            VoteCmp::Geq,
            Uint128::new(0),
            Decimal::permille(500)
        ));
        assert!(!compare_vote_count(
            Uint128::new(0),
            VoteCmp::Greater,
            Uint128::new(0),
            Decimal::permille(500)
        ));

        assert!(!compare_vote_count(
            Uint128::new(0),
            VoteCmp::Geq,
            Uint128::new(1),
            Decimal::permille(1)
        ));
        assert!(!compare_vote_count(
            Uint128::new(0),
            VoteCmp::Greater,
            Uint128::new(1),
            Decimal::permille(1)
        ));

        assert!(compare_vote_count(
            Uint128::new(1),
            VoteCmp::Geq,
            Uint128::new(1),
            Decimal::permille(1)
        ));
        assert!(compare_vote_count(
            Uint128::new(1),
            VoteCmp::Greater,
            Uint128::new(1),
            Decimal::permille(1)
        ));

        assert!(!compare_vote_count(
            Uint128::new(0),
            VoteCmp::Geq,
            Uint128::new(1),
            Decimal::permille(999)
        ));
        assert!(!compare_vote_count(
            Uint128::new(0),
            VoteCmp::Greater,
            Uint128::new(1),
            Decimal::permille(999)
        ));
    }

    #[test]
    fn tricky_vote_counts() {
        let threshold = Decimal::percent(50);
        for count in 1..50_000 {
            assert!(compare_vote_count(
                Uint128::new(count),
                VoteCmp::Geq,
                Uint128::new(count * 2),
                threshold
            ));
            assert!(!compare_vote_count(
                Uint128::new(count),
                VoteCmp::Greater,
                Uint128::new(count * 2),
                threshold
            ))
        }

        // Zero votes out of zero total power meet any threshold. When
        // Geq is used. Always fail otherwise.
        assert!(compare_vote_count(
            Uint128::zero(),
            VoteCmp::Geq,
            Uint128::new(1),
            Decimal::percent(0)
        ));
        assert!(compare_vote_count(
            Uint128::zero(),
            VoteCmp::Geq,
            Uint128::new(0),
            Decimal::percent(0)
        ));
        assert!(!compare_vote_count(
            Uint128::zero(),
            VoteCmp::Greater,
            Uint128::new(1),
            Decimal::percent(0)
        ));
        assert!(!compare_vote_count(
            Uint128::zero(),
            VoteCmp::Greater,
            Uint128::new(0),
            Decimal::percent(0)
        ))
    }
}
