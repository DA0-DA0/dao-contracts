use cosmwasm_std::Uint128;
use cw_utils::Expiration;

use crate::{
    tally::{Tally, Winner},
    vote::Vote,
};

#[test]
fn test_pair_election() {
    let candidates = 2;
    let mut tally = Tally::new(candidates, Uint128::new(3), 0, Expiration::Never {});

    tally.add_vote(Vote::new(vec![0, 1], candidates).unwrap(), Uint128::one());
    tally.add_vote(Vote::new(vec![1, 0], candidates).unwrap(), Uint128::one());
    tally.add_vote(Vote::new(vec![1, 0], candidates).unwrap(), Uint128::one());

    assert_eq!(tally.winner, Winner::Undisputed(1));
}

#[test]
fn test_triplet_election() {
    let candidates = 3;
    let mut tally = Tally::new(candidates, Uint128::new(3), 0, Expiration::Never {});

    tally.add_vote(
        Vote::new(vec![0, 1, 2], candidates).unwrap(),
        Uint128::one(),
    );

    assert_eq!(tally.winner, Winner::Some(0));

    tally.add_vote(
        Vote::new(vec![0, 2, 1], candidates).unwrap(),
        Uint128::one(),
    );
    tally.add_vote(
        Vote::new(vec![2, 0, 1], candidates).unwrap(),
        Uint128::one(),
    );

    assert_eq!(tally.winner, Winner::Undisputed(0));
}

#[test]
fn test_condorcet_paradox() {
    let candidates = 3;
    let mut tally = Tally::new(candidates, Uint128::new(6), 0, Expiration::Never {});

    tally.add_vote(
        Vote::new(vec![0, 2, 1], candidates).unwrap(),
        Uint128::one(),
    );
    tally.add_vote(
        Vote::new(vec![1, 0, 2], candidates).unwrap(),
        Uint128::one(),
    );
    tally.add_vote(
        Vote::new(vec![2, 1, 0], candidates).unwrap(),
        Uint128::one(),
    );
    tally.add_vote(
        Vote::new(vec![1, 0, 2], candidates).unwrap(),
        Uint128::one(),
    );
    tally.add_vote(
        Vote::new(vec![0, 2, 1], candidates).unwrap(),
        Uint128::one(),
    );
    tally.add_vote(
        Vote::new(vec![2, 0, 1], candidates).unwrap(),
        Uint128::one(),
    );

    // sequence of ballots cast:
    //
    // 0 > 2 > 1
    // 1 > 0 > 2
    // 2 > 1 > 0
    // 1 > 0 > 2
    // 0 > 2 > 1
    // 2 > 0 > 1
    //
    // produces a M matrix:
    //
    // ```
    //   \  0 -2
    //   0  \  2
    //   2 -2  \
    // ```
    //
    // the "condorcet paradox" 0 > 2, 2 > 1, 0 !> 1.
    assert_eq!(tally.winner, Winner::Never)
}

#[test]
fn test_tally_overflow() {
    let candidates = 6;
    let mut tally = Tally::new(candidates, Uint128::MAX, 0, Expiration::Never {});

    tally.add_vote(
        Vote::new(vec![1, 2, 3, 4, 5, 0], candidates).unwrap(),
        Uint128::new(u128::MAX / 2),
    );
    tally.add_vote(
        Vote::new(vec![2, 1, 3, 5, 0, 4], candidates).unwrap(),
        Uint128::new(u128::MAX / 2 - 1),
    );
    tally.add_vote(
        Vote::new(vec![5, 0, 3, 1, 2, 4], candidates).unwrap(),
        Uint128::one(),
    );

    assert_eq!(tally.winner, Winner::Undisputed(1))
}

#[test]
fn test_winner_none() {
    let candidates = 6;
    let mut tally = Tally::new(candidates, Uint128::new(9), 0, Expiration::Never {});

    tally.add_vote(
        Vote::new(vec![1, 2, 3, 4, 5, 0], candidates).unwrap(),
        Uint128::new(2),
    );

    tally.add_vote(
        Vote::new(vec![4, 5, 3, 0, 2, 1], candidates).unwrap(),
        Uint128::new(2),
    );

    tally.add_vote(
        Vote::new(vec![2, 3, 0, 5, 4, 1], candidates).unwrap(),
        Uint128::new(1),
    );

    tally.add_vote(
        Vote::new(vec![3, 0, 2, 4, 5, 1], candidates).unwrap(),
        Uint128::new(1),
    );

    // at this point, there is no winner, but there is three voting
    // power outstanding and 6 candidates so column 2, 3, etc. could
    // be flipped so we can't declare the election over.
    //
    //  \ -2  0  6  2  2
    //  2  \  2  2  2  2
    //  0 -2  \  0 -2 -2
    // -6 -2  0  \ -2 -2
    // -2 -2  2  2  \ -4
    // -2 -2  2  2  4  \
    assert_eq!(tally.winner, Winner::None)
}
