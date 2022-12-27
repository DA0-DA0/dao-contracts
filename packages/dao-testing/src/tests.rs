use cosmwasm_std::{Decimal, Uint128};
use dao_voting::status::Status;
use dao_voting::threshold::{PercentageThreshold, Threshold};
use dao_voting::voting::Vote;
use rand::{prelude::SliceRandom, Rng};

/// If a test vote should execute. Used for fuzzing and checking that
/// votes after a proposal has completed aren't allowed.
pub enum ShouldExecute {
    /// This should execute.
    Yes,
    /// This should not execute.
    No,
    /// Doesn't matter.
    Meh,
}

pub struct TestSingleChoiceVote {
    /// The address casting the vote.
    pub voter: String,
    /// Position on the vote.
    pub position: Vote,
    /// Voting power of the address.
    pub weight: Uint128,
    /// If this vote is expected to execute.
    pub should_execute: ShouldExecute,
}

pub fn test_simple_votes<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Passed,
        None,
    );

    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Rejected,
        None,
    )
}

pub fn test_simple_vote_no_overflow<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(u128::max_value()),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Passed,
        None,
    );
}

pub fn test_vote_no_overflow<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(u128::max_value()),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Passed,
        None,
    );

    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "zeke".to_string(),
                position: Vote::No,
                weight: Uint128::new(1),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(u128::max_value() - 1),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(99)),
        },
        Status::Passed,
        None,
    )
}

pub fn test_simple_early_rejection<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "zeke".to_string(),
            position: Vote::No,
            weight: Uint128::new(1),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Rejected,
        None,
    );

    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(1),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(99)),
        },
        Status::Open,
        Some(Uint128::from(u128::max_value())),
    );
}

pub fn test_vote_abstain_only<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Abstain,
            weight: Uint128::new(u64::max_value().into()),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Rejected,
        None,
    );

    // The quorum shouldn't matter here in determining if the vote is
    // rejected.
    for i in 0..101 {
        do_votes(
            vec![TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(u64::max_value().into()),
                should_execute: ShouldExecute::Yes,
            }],
            Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Percent(Decimal::percent(100)),
                quorum: PercentageThreshold::Percent(Decimal::percent(i)),
            },
            Status::Rejected,
            None,
        );
    }
}

pub fn test_tricky_rounding<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    // This tests the smallest possible round up for passing
    // thresholds we can have. Specifically, a 1% passing threshold
    // and 1 total vote. This should round up and only pass if there
    // are more than 1 yes votes.
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(1),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(1)),
        },
        Status::Passed,
        Some(Uint128::new(100)),
    );

    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(1)),
        },
        Status::Passed,
        Some(Uint128::new(1000)),
    );

    // HIGH PERCISION
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(9999999),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(1)),
        },
        Status::Open,
        Some(Uint128::new(1000000000)),
    );

    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Abstain,
            weight: Uint128::new(1),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(1)),
        },
        Status::Rejected,
        None,
    );
}

pub fn test_no_double_votes<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(2),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(2),
                should_execute: ShouldExecute::No,
            },
        ],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        // NOTE: Updating our cw20-base version will cause this to
        // fail. In versions of cw20-base before Feb 15 2022 (the one
        // we use at the time of writing) it was allowed to have an
        // initial balance that repeats for a given address but it
        // would cause miscalculation of the total supply. In this
        // case the total supply is miscumputed to be 4 so this is
        // assumed to have 2 abstain votes out of 4 possible votes.
        Status::Open,
        Some(Uint128::new(10)),
    )
}

pub fn test_votes_favor_yes<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::No,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "ezek".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(50)),
        },
        Status::Passed,
        None,
    );

    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(50)),
        },
        Status::Passed,
        None,
    );

    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
            // Can vote up to expiration time.
            TestSingleChoiceVote {
                voter: "ezek".to_string(),
                position: Vote::No,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(50)),
        },
        Status::Passed,
        None,
    );
}

pub fn test_votes_low_threshold<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Percent(Decimal::percent(10)),
            quorum: PercentageThreshold::Majority {},
        },
        Status::Passed,
        None,
    );

    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
            // Can vote up to expiration time.
            TestSingleChoiceVote {
                voter: "ezek".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Percent(Decimal::percent(10)),
            quorum: PercentageThreshold::Majority {},
        },
        Status::Passed,
        None,
    );
}

pub fn test_majority_vs_half<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Percent(Decimal::percent(50)),
            quorum: PercentageThreshold::Majority {},
        },
        Status::Passed,
        None,
    );

    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            // Can vote up to expiration time, even if it already rejected.
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Majority {},
        },
        Status::Rejected,
        None,
    );
}

pub fn test_pass_threshold_not_quorum<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(59),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(60)),
        },
        Status::Open,
        Some(Uint128::new(100)),
    );
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(59),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(60)),
        },
        // As the threshold is 50% and 59% of voters have voted no
        // this is unable to pass.
        Status::Rejected,
        Some(Uint128::new(100)),
    );
}

pub fn test_pass_exactly_quorum<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(60),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(60)),
        },
        Status::Passed,
        Some(Uint128::new(100)),
    );
    do_votes(
        vec![
            TestSingleChoiceVote {
                voter: "ekez".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(59),
                should_execute: ShouldExecute::Yes,
            },
            // This is an intersting one because in this case the no
            // voter is actually incentivised not to vote. By voting
            // they move the quorum over the threshold and pass the
            // vote. In a DAO with sufficently involved stakeholders
            // no voters should effectively never vote if there is a
            // quorum higher than the threshold as it makes the
            // passing threshold the quorum threshold.
            TestSingleChoiceVote {
                voter: "keze".to_string(),
                position: Vote::No,
                weight: Uint128::new(1),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(60)),
        },
        Status::Passed,
        Some(Uint128::new(100)),
    );
    do_votes(
        vec![TestSingleChoiceVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(60),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(60)),
        },
        Status::Rejected,
        Some(Uint128::new(100)),
    );
}

pub fn fuzz_voting<F>(do_votes: F)
where
    F: Fn(Vec<TestSingleChoiceVote>, Threshold, Status, Option<Uint128>),
{
    let mut rng = rand::thread_rng();
    let dist = rand::distributions::Uniform::<u64>::new(1, 200);
    for _ in 0..10 {
        let yes: Vec<u64> = (0..50).map(|_| rng.sample(dist)).collect();
        let no: Vec<u64> = (0..50).map(|_| rng.sample(dist)).collect();

        let yes_sum: u64 = yes.iter().sum();
        let no_sum: u64 = no.iter().sum();
        let expected_status = match yes_sum.cmp(&no_sum) {
            std::cmp::Ordering::Less => Status::Rejected,
            // Depends on which reaches the threshold first. Ignore for now.
            std::cmp::Ordering::Equal => Status::Rejected,
            std::cmp::Ordering::Greater => Status::Passed,
        };

        let yes = yes
            .into_iter()
            .enumerate()
            .map(|(idx, weight)| TestSingleChoiceVote {
                voter: format!("yes_{idx}"),
                position: Vote::Yes,
                weight: Uint128::new(weight as u128),
                should_execute: ShouldExecute::Meh,
            });
        let no = no
            .into_iter()
            .enumerate()
            .map(|(idx, weight)| TestSingleChoiceVote {
                voter: format!("no_{idx}"),
                position: Vote::No,
                weight: Uint128::new(weight as u128),
                should_execute: ShouldExecute::Meh,
            });
        let mut votes = yes.chain(no).collect::<Vec<_>>();
        votes.shuffle(&mut rng);

        do_votes(
            votes,
            Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            expected_status,
            None,
        );
    }
}
