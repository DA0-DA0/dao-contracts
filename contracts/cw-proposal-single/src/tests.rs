use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use indexable_hooks::HooksResponse;
use rand::{prelude::SliceRandom, Rng};

use voting::{Vote, Votes};

use crate::{
    msg::{DepositInfo, DepositToken, ExecuteMsg, InstantiateMsg, QueryMsg},
    proposal::{Proposal, Status},
    query::{ProposalListResponse, ProposalResponse, VoteInfo, VoteResponse},
    state::{CheckedDepositInfo, Config},
    threshold::{PercentageThreshold, Threshold},
};

const CREATOR_ADDR: &str = "creator";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn single_govmod_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

fn cw20_balances_voting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_balance_voting::contract::execute,
        cw20_balance_voting::contract::instantiate,
        cw20_balance_voting::contract::query,
    )
    .with_reply(cw20_balance_voting::contract::reply);
    Box::new(contract)
}

fn cw_gov_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_core::contract::execute,
        cw_core::contract::instantiate,
        cw_core::contract::query,
    )
    .with_reply(cw_core::contract::reply);
    Box::new(contract)
}

fn instantiate_governance(app: &mut App, code_id: u64, msg: cw_core::msg::InstantiateMsg) -> Addr {
    app.instantiate_contract(
        code_id,
        Addr::unchecked(CREATOR_ADDR),
        &msg,
        &[],
        "cw-governance",
        None,
    )
    .unwrap()
}

fn instantiate_with_default_governance(
    app: &mut App,
    code_id: u64,
    msg: InstantiateMsg,
    initial_balances: Option<Vec<Cw20Coin>>,
) -> Addr {
    let cw20_id = app.store_code(cw20_contract());
    let governance_id = app.store_code(cw_gov_contract());
    let votemod_id = app.store_code(cw20_balances_voting());

    let initial_balances = initial_balances.unwrap_or_else(|| {
        vec![Cw20Coin {
            address: CREATOR_ADDR.to_string(),
            amount: Uint128::new(100_000_000),
        }]
    });

    let governance_instantiate = cw_core::msg::InstantiateMsg {
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: cw_core::msg::ModuleInstantiateInfo {
            code_id: votemod_id,
            msg: to_binary(&cw20_balance_voting::msg::InstantiateMsg {
                token_info: cw20_balance_voting::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token".to_string(),
                    name: "DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances,
                    marketing: None,
                },
            })
            .unwrap(),
            admin: cw_core::msg::Admin::GovernanceContract {},
            label: "DAO DAO voting module".to_string(),
        },
        governance_modules_instantiate_info: vec![cw_core::msg::ModuleInstantiateInfo {
            code_id,
            msg: to_binary(&msg).unwrap(),
            admin: cw_core::msg::Admin::GovernanceContract {},
            label: "DAO DAO governance module".to_string(),
        }],
        initial_items: None,
    };

    instantiate_governance(app, governance_id, governance_instantiate)
}

#[test]
fn test_propose() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold: threshold.clone(),
        max_voting_period,
        only_members_execute: false,
        deposit_info: None,
    };

    let governance_addr =
        instantiate_with_default_governance(&mut app, govmod_id, instantiate, None);
    let governance_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            governance_addr.clone(),
            &cw_core::msg::QueryMsg::GovernanceModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    // Check that the governance module has been configured correctly.
    let config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let expected = Config {
        threshold: threshold.clone(),
        max_voting_period,
        only_members_execute: false,
        dao: governance_addr,
        deposit_info: None,
    };
    assert_eq!(config, expected);

    // Create a new proposal.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Propose {
            title: "A simple text proposal".to_string(),
            description: "This is a simple text proposal".to_string(),
            msgs: vec![],
        },
        &[],
    )
    .unwrap();

    let created: ProposalResponse = app
        .wrap()
        .query_wasm_smart(govmod_single, &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    let current_block = app.block_info();
    let expected = Proposal {
        title: "A simple text proposal".to_string(),
        description: "This is a simple text proposal".to_string(),
        proposer: Addr::unchecked(CREATOR_ADDR),
        start_height: current_block.height,
        expiration: max_voting_period.after(&current_block),
        threshold,
        total_power: Uint128::new(100_000_000),
        msgs: vec![],
        status: crate::proposal::Status::Open,
        votes: Votes::zero(),
        deposit_info: None,
    };

    assert_eq!(created.proposal, expected);
    assert_eq!(created.id, 1u64);
}

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

struct TestVote {
    /// The address casting the vote.
    voter: String,
    /// Position on the vote.
    position: Vote,
    /// Voting power of the address.
    weight: Uint128,
    /// If this vote is expected to execute.
    should_execute: ShouldExecute,
}

// Creates a proposal and then executes a series of votes on those
// proposals. Asserts both that those votes execute as expected and
// that the final status of the proposal is what is expected. Returns
// the address of the governance contract that it has created so that
// callers may do additional inspection of the contract's state.
fn do_test_votes(
    votes: Vec<TestVote>,
    threshold: Threshold,
    expected_status: Status,
    total_supply: Option<Uint128>,
    deposit_info: Option<DepositInfo>,
) -> (App, Addr) {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());

    let mut initial_balances = votes
        .iter()
        .map(|TestVote { voter, weight, .. }| Cw20Coin {
            address: voter.to_string(),
            amount: *weight,
        })
        .collect::<Vec<Cw20Coin>>();
    let initial_balances_supply = votes.iter().fold(Uint128::zero(), |p, n| p + n.weight);
    let to_fill = total_supply.map(|total_supply| total_supply - initial_balances_supply);
    if let Some(fill) = to_fill {
        initial_balances.push(Cw20Coin {
            address: "filler".to_string(),
            amount: fill,
        })
    }

    let proposer = match votes.first() {
        Some(vote) => vote.voter.clone(),
        None => panic!("do_test_votes must have at least one vote."),
    };

    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        only_members_execute: false,
        deposit_info,
    };

    let governance_addr = instantiate_with_default_governance(
        &mut app,
        govmod_id,
        instantiate,
        Some(initial_balances),
    );

    let governance_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            governance_addr.clone(),
            &cw_core::msg::QueryMsg::GovernanceModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    // Allow a proposal deposit as needed.
    let config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    if let Some(CheckedDepositInfo {
        ref token, deposit, ..
    }) = config.deposit_info
    {
        app.execute_contract(
            Addr::unchecked(&proposer),
            token.clone(),
            &cw20_base::msg::ExecuteMsg::IncreaseAllowance {
                spender: govmod_single.to_string(),
                amount: deposit,
                expires: None,
            },
            &[],
        )
        .unwrap();
    }

    app.execute_contract(
        Addr::unchecked(&proposer),
        govmod_single.clone(),
        &ExecuteMsg::Propose {
            title: "A simple text proposal".to_string(),
            description: "This is a simple text proposal".to_string(),
            msgs: vec![],
        },
        &[],
    )
    .unwrap();

    // Cast votes.
    for vote in votes {
        let TestVote {
            voter,
            position,
            weight,
            should_execute,
        } = vote;
        // Vote on the proposal.
        let res = app.execute_contract(
            Addr::unchecked(voter.clone()),
            govmod_single.clone(),
            &ExecuteMsg::Vote {
                proposal_id: 1,
                vote: position,
            },
            &[],
        );
        match should_execute {
            ShouldExecute::Yes => {
                assert!(res.is_ok());
                // Check that the vote was recorded correctly.
                let vote: VoteResponse = app
                    .wrap()
                    .query_wasm_smart(
                        govmod_single.clone(),
                        &QueryMsg::Vote {
                            proposal_id: 1,
                            voter: voter.clone(),
                        },
                    )
                    .unwrap();
                let expected = VoteResponse {
                    vote: Some(VoteInfo {
                        voter: Addr::unchecked(&voter),
                        vote: position,
                        power: match config.deposit_info {
                            Some(CheckedDepositInfo { deposit, .. }) => {
                                if proposer == voter {
                                    weight - deposit
                                } else {
                                    weight
                                }
                            }
                            None => weight,
                        },
                    }),
                };
                assert_eq!(vote, expected)
            }
            ShouldExecute::No => assert!(res.is_err()),
            ShouldExecute::Meh => (),
        }
    }

    let proposal: ProposalResponse = app
        .wrap()
        .query_wasm_smart(govmod_single, &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();

    assert_eq!(proposal.proposal.status, expected_status);

    (app, governance_addr)
}

#[test]
fn test_vote_simple() {
    do_test_votes(
        vec![TestVote {
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
        None,
    );

    do_test_votes(
        vec![TestVote {
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
        None,
    );
}

#[test]
fn test_vote_no_overflow() {
    // We should not overflow when computing passing thresholds even
    // when there are 2^128 votes.
    do_test_votes(
        vec![TestVote {
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
        None,
    );
}

#[test]
fn test_vote_abstain_only() {
    do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::Abstain,
            weight: Uint128::new(u128::max_value()),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Rejected,
        None,
        None,
    );

    // The quorum shouldn't matter here in determining if the vote is
    // rejected.
    for i in 0..101 {
        do_test_votes(
            vec![TestVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(u128::max_value()),
                should_execute: ShouldExecute::Yes,
            }],
            Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Percent(Decimal::percent(100)),
                quorum: PercentageThreshold::Percent(Decimal::percent(i)),
            },
            Status::Rejected,
            None,
            None,
        );
    }
}

#[test]
fn test_single_no() {
    do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(1),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(100)),
        },
        Status::Rejected,
        Some(Uint128::from(u128::max_value())),
        None,
    );
}

#[test]
fn test_tricky_rounding() {
    // This tests the smallest possible round up for passing
    // thresholds we can have. Specifically, a 1% passing threshold
    // and 1 total vote. This should round up and only pass if there
    // are more than 1 yes votes.
    do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(1),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(1)),
        },
        Status::Passed,
        None,
        None,
    );

    do_test_votes(
        vec![TestVote {
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
        None,
    );
}

#[test]
fn test_no_double_votes() {
    do_test_votes(
        vec![
            TestVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(2),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
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
        None,
        None,
    );
}

/// The current behavior for passing proposals is that the first
/// option to reach the threshold wins. For example, with a 50%
/// passing threshold if 50% of voting power votes no that proposal
/// fails even if the other 50% would have voted yes. The same goes if
/// the yes and no were reversed.
///
/// TODO(zeke): is this the behavior that we want?
#[test]
fn test_close_votes() {
    do_test_votes(
        vec![
            TestVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
                voter: "keze".to_string(),
                position: Vote::No,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
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
        None,
    );

    do_test_votes(
        vec![
            TestVote {
                voter: "ekez".to_string(),
                position: Vote::Abstain,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
                voter: "ezek".to_string(),
                position: Vote::No,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::No,
            },
        ],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(50)),
        },
        Status::Passed,
        None,
        None,
    );
}

/// Another test which demonstrates the trouble with our "first to
/// reach threshold" method for determining the winner. In this case
/// there are more no votes than yes votes but because yes votes are
/// the first ones to reach the threshold after the quorum has been
/// passed yes votes win.
///
/// This is a pretty nonsense passing threshold but helps demonstrate
/// the issue well enough.
#[test]
fn test_close_votes_quorum() {
    do_test_votes(
        vec![
            TestVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(5),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
                voter: "ezek".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::No,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Percent(Decimal::percent(10)),
            quorum: PercentageThreshold::Majority {},
        },
        Status::Passed,
        None,
        None,
    );
}

#[test]
fn test_majority_vs_half() {
    do_test_votes(
        vec![
            TestVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
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
        None,
    );

    do_test_votes(
        vec![
            TestVote {
                voter: "ekez".to_string(),
                position: Vote::No,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestVote {
                voter: "keze".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::No,
            },
        ],
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Majority {},
        },
        Status::Rejected,
        None,
        None,
    );
}

#[test]
fn test_pass_threshold_not_quorum() {
    do_test_votes(
        vec![TestVote {
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
        None,
    );
    do_test_votes(
        vec![TestVote {
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
        None,
    );
}

#[test]
fn test_pass_threshold_exactly_quorum() {
    do_test_votes(
        vec![TestVote {
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
        None,
    );
    do_test_votes(
        vec![
            TestVote {
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
            TestVote {
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
        None,
    );
    do_test_votes(
        vec![TestVote {
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
        None,
    );
}

/// Generate some random voting selections and make sure they behave
/// as expected.
#[test]
fn fuzz_voting() {
    let mut rng = rand::thread_rng();
    let dist = rand::distributions::Uniform::<u64>::new(1, 200);
    for _ in 0..25 {
        let yes: Vec<u64> = (0..50).map(|_| rng.sample(&dist)).collect();
        let no: Vec<u64> = (0..50).map(|_| rng.sample(&dist)).collect();

        let yes_sum: u64 = yes.iter().sum();
        let no_sum: u64 = no.iter().sum();
        let expected_status = match yes_sum.cmp(&no_sum) {
            std::cmp::Ordering::Less => Status::Rejected,
            // Depends on which reaches the threshold first. Ignore for now.
            std::cmp::Ordering::Equal => Status::Rejected,
            std::cmp::Ordering::Greater => Status::Passed,
        };

        let yes = yes.into_iter().enumerate().map(|(idx, weight)| TestVote {
            voter: format!("yes_{}", idx),
            position: Vote::Yes,
            weight: Uint128::new(weight as u128),
            should_execute: ShouldExecute::Meh,
        });
        let no = no.into_iter().enumerate().map(|(idx, weight)| TestVote {
            voter: format!("no_{}", idx),
            position: Vote::No,
            weight: Uint128::new(weight as u128),
            should_execute: ShouldExecute::Meh,
        });
        let mut votes = yes.chain(no).collect::<Vec<_>>();
        votes.shuffle(&mut rng);

        do_test_votes(
            votes,
            Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            expected_status,
            None,
            None,
        );
    }
}

/// Instantiate the contract and use the voting module's token
/// contract as the proposal deposit token.
#[test]
fn test_voting_module_token_proposal_deposit_instantiate() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        only_members_execute: false,
        deposit_info: Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: true,
        }),
    };

    let governance_addr =
        instantiate_with_default_governance(&mut app, govmod_id, instantiate, None);

    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;
    let voting_module = gov_state.voting_module;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single, &QueryMsg::Config {})
        .unwrap();
    let expected_token: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cw_core_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    assert_eq!(
        config.deposit_info,
        Some(CheckedDepositInfo {
            token: expected_token,
            deposit: Uint128::new(1),
            refund_failed_proposals: true
        })
    )
}

/// Instantiate the contract and use a cw20 unrealated to the voting
/// module for the proposal deposit.
#[test]
fn test_different_token_proposal_deposit() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());
    let cw20_id = app.store_code(cw20_contract());
    let cw20_addr = app
        .instantiate_contract(
            cw20_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "OAD OAD".to_string(),
                symbol: "OAD".to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: None,
                marketing: None,
            },
            &[],
            "random-cw20",
            None,
        )
        .unwrap();

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        only_members_execute: false,
        deposit_info: Some(DepositInfo {
            token: DepositToken::Token {
                address: cw20_addr.to_string(),
            },
            deposit: Uint128::new(1),
            refund_failed_proposals: true,
        }),
    };

    instantiate_with_default_governance(&mut app, govmod_id, instantiate, None);
}

/// Try to instantiate the governance module with a non-cw20 as its
/// proposal deposit token. This should error as the `TokenInfo {}`
/// query ought to fail.
#[test]
#[should_panic(expected = "Error parsing into type cw20_balance_voting::msg::QueryMsg")]
fn test_bad_token_proposal_deposit() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());
    let cw20_id = app.store_code(cw20_contract());
    let votemod_id = app.store_code(cw20_balances_voting());

    let votemod_addr = app
        .instantiate_contract(
            votemod_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_balance_voting::msg::InstantiateMsg {
                token_info: cw20_balance_voting::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token".to_string(),
                    name: "DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: CREATOR_ADDR.to_string(),
                        amount: Uint128::new(1),
                    }],
                    marketing: None,
                },
            },
            &[],
            "random-vote-module",
            None,
        )
        .unwrap();

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        only_members_execute: false,
        deposit_info: Some(DepositInfo {
            token: DepositToken::Token {
                address: votemod_addr.to_string(),
            },
            deposit: Uint128::new(1),
            refund_failed_proposals: true,
        }),
    };

    instantiate_with_default_governance(&mut app, govmod_id, instantiate, None);
}

#[test]
fn test_take_proposal_deposit() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        only_members_execute: false,
        deposit_info: Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: true,
        }),
    };

    let governance_addr = instantiate_with_default_governance(
        &mut app,
        govmod_id,
        instantiate,
        Some(vec![Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(2),
        }]),
    );

    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let CheckedDepositInfo {
        token,
        deposit,
        refund_failed_proposals,
    } = govmod_config.deposit_info.unwrap();
    assert!(refund_failed_proposals);
    assert_eq!(deposit, Uint128::new(1));

    // This should fail because we have not created an allowance for
    // the proposal deposit.
    app.execute_contract(
        Addr::unchecked("ekez"),
        govmod_single.clone(),
        &ExecuteMsg::Propose {
            title: "A simple text proposal".to_string(),
            description: "This is a simple text proposal".to_string(),
            msgs: vec![],
        },
        &[],
    )
    .unwrap_err();

    // Allow a proposal deposit.
    app.execute_contract(
        Addr::unchecked("ekez"),
        token.clone(),
        &cw20_base::msg::ExecuteMsg::IncreaseAllowance {
            spender: govmod_single.to_string(),
            amount: Uint128::new(1),
            expires: None,
        },
        &[],
    )
    .unwrap();

    // Now we can create a proposal.
    app.execute_contract(
        Addr::unchecked("ekez"),
        govmod_single,
        &ExecuteMsg::Propose {
            title: "A simple text proposal".to_string(),
            description: "This is a simple text proposal".to_string(),
            msgs: vec![],
        },
        &[],
    )
    .unwrap();

    // Check that our balance was deducted.
    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();
    assert_eq!(balance.balance, Uint128::new(1))
}

#[test]
fn test_deposit_return_on_execute() {
    // Will create a proposal and execute it, one token will be
    // deposited to create said proposal, expectation is that the
    // token is then returned once the proposal is executed.
    let (mut app, governance_addr) = do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        },
        Status::Passed,
        None,
        Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: false,
        }),
    );
    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let CheckedDepositInfo { token, .. } = govmod_config.deposit_info.unwrap();
    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();

    // Proposal has not been executed so deposit has not been
    // refunded.
    assert_eq!(balance.balance, Uint128::new(9));

    // Execute the proposal, this should cause the deposit to be
    // refunded.
    app.execute_contract(
        Addr::unchecked("ekez"),
        govmod_single,
        &ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();

    // Proposal has been executed so deposit has been refunded.
    assert_eq!(balance.balance, Uint128::new(10));
}

#[test]
fn test_close_open_proposal() {
    let (mut app, governance_addr) = do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        },
        Status::Open,
        Some(Uint128::new(100)),
        Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: true,
        }),
    );

    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    // Close the proposal, this should error as the proposal is still
    // open and not expired.
    app.execute_contract(
        Addr::unchecked("keze"),
        govmod_single.clone(),
        &ExecuteMsg::Close { proposal_id: 1 },
        &[],
    )
    .unwrap_err();

    // Make the proposal expire.
    app.update_block(|block| block.height += 10);

    // Close the proposal, this should work as the proposal is now
    // open and expired.
    app.execute_contract(
        Addr::unchecked("keze"),
        govmod_single.clone(),
        &ExecuteMsg::Close { proposal_id: 1 },
        &[],
    )
    .unwrap();

    // Check that a refund was issued.
    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single, &QueryMsg::Config {})
        .unwrap();
    let CheckedDepositInfo { token, .. } = govmod_config.deposit_info.unwrap();
    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();

    // Proposal has not been closed so deposit has not been
    // refunded.
    assert_eq!(balance.balance, Uint128::new(10));
}

#[test]
fn test_zero_deposit() {
    do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::Yes,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        },
        Status::Passed,
        None,
        Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(0),
            refund_failed_proposals: false,
        }),
    );
}

#[test]
fn test_deposit_return_on_close() {
    let (mut app, governance_addr) = do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        },
        Status::Rejected,
        None,
        Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: true,
        }),
    );
    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let CheckedDepositInfo { token, .. } = govmod_config.deposit_info.unwrap();
    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();

    // Proposal has not been closed so deposit has not been
    // refunded.
    assert_eq!(balance.balance, Uint128::new(9));

    // Close the proposal, this should cause the deposit to be
    // refunded.
    app.execute_contract(
        Addr::unchecked("ekez"),
        govmod_single,
        &ExecuteMsg::Close { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();

    // Proposal has been closed so deposit has been refunded.
    assert_eq!(balance.balance, Uint128::new(10));
}

#[test]
fn test_update_config() {
    let (mut app, governance_addr) = do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        },
        Status::Rejected,
        None,
        Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: false,
        }),
    );

    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(
        govmod_config.threshold,
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        }
    );

    let dao = govmod_config.dao;

    // Attempt to update the config from a non-dao address. This
    // should fail as it is unauthorized.
    app.execute_contract(
        Addr::unchecked("ekez"),
        govmod_single.clone(),
        &ExecuteMsg::UpdateConfig {
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Height(10),
            only_members_execute: false,
            dao: CREATOR_ADDR.to_string(),
            deposit_info: None,
        },
        &[],
    )
    .unwrap_err();

    // Update the config from the DAO address. This should succede.
    app.execute_contract(
        dao.clone(),
        govmod_single.clone(),
        &ExecuteMsg::UpdateConfig {
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Height(10),
            only_members_execute: false,
            dao: CREATOR_ADDR.to_string(),
            deposit_info: None,
        },
        &[],
    )
    .unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();

    let expected = Config {
        threshold: Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Majority {},
        },
        max_voting_period: cw_utils::Duration::Height(10),
        only_members_execute: false,
        dao: Addr::unchecked(CREATOR_ADDR),
        deposit_info: None,
    };
    assert_eq!(govmod_config, expected);

    // As we have changed the DAO address updating the config using
    // the original one should now fail.
    app.execute_contract(
        dao,
        govmod_single,
        &ExecuteMsg::UpdateConfig {
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Height(10),
            only_members_execute: false,
            dao: CREATOR_ADDR.to_string(),
            deposit_info: None,
        },
        &[],
    )
    .unwrap_err();
}

#[test]
fn test_no_return_if_no_refunds() {
    let (mut app, governance_addr) = do_test_votes(
        vec![TestVote {
            voter: "ekez".to_string(),
            position: Vote::No,
            weight: Uint128::new(10),
            should_execute: ShouldExecute::Yes,
        }],
        Threshold::AbsolutePercentage {
            percentage: PercentageThreshold::Percent(Decimal::percent(90)),
        },
        Status::Rejected,
        None,
        Some(DepositInfo {
            token: DepositToken::VotingModuleToken {},
            deposit: Uint128::new(1),
            refund_failed_proposals: false,
        }),
    );
    let gov_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(governance_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let governance_modules = gov_state.governance_modules;

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let CheckedDepositInfo { token, .. } = govmod_config.deposit_info.unwrap();

    // Close the proposal, this should cause the deposit to be
    // refunded.
    app.execute_contract(
        Addr::unchecked("ekez"),
        govmod_single,
        &ExecuteMsg::Close { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: "ekez".to_string(),
            },
        )
        .unwrap();

    // Proposal has been closed but deposit has not been refunded.
    assert_eq!(balance.balance, Uint128::new(9));
}

#[test]
fn test_query_list_proposals() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());
    let gov_addr = instantiate_with_default_governance(
        &mut app,
        govmod_id,
        InstantiateMsg {
            threshold: Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Majority {},
                quorum: PercentageThreshold::Percent(Decimal::percent(0)),
            },
            max_voting_period: cw_utils::Duration::Height(100),
            only_members_execute: true,
            deposit_info: None,
        },
        Some(vec![Cw20Coin {
            address: CREATOR_ADDR.to_string(),
            amount: Uint128::new(100),
        }]),
    );

    let gov_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &cw_core::msg::QueryMsg::GovernanceModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(gov_modules.len(), 1);

    let govmod = gov_modules.into_iter().next().unwrap();

    for i in 1..10 {
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod.clone(),
            &ExecuteMsg::Propose {
                title: format!("Text proposal {}.", i),
                description: "This is a simple text proposal".to_string(),
                msgs: vec![],
            },
            &[],
        )
        .unwrap();
    }

    let proposals_forward: ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            govmod.clone(),
            &QueryMsg::ListProposals {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    let mut proposals_backward: ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            govmod.clone(),
            &QueryMsg::ReverseProposals {
                start_before: None,
                limit: None,
            },
        )
        .unwrap();

    proposals_backward.proposals.reverse();

    assert_eq!(proposals_forward.proposals, proposals_backward.proposals);

    let expected = ProposalResponse {
        id: 1,
        proposal: Proposal {
            title: "Text proposal 1.".to_string(),
            description: "This is a simple text proposal".to_string(),
            proposer: Addr::unchecked(CREATOR_ADDR),
            start_height: app.block_info().height,
            expiration: cw_utils::Expiration::AtHeight(app.block_info().height + 100),
            threshold: Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Majority {},
                quorum: PercentageThreshold::Percent(Decimal::percent(0)),
            },
            total_power: Uint128::new(100),
            msgs: vec![],
            status: Status::Open,
            votes: Votes::zero(),
            deposit_info: None,
        },
    };
    assert_eq!(proposals_forward.proposals[0], expected);

    // Get proposals (3, 5]
    let proposals_forward: ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            govmod.clone(),
            &QueryMsg::ListProposals {
                start_after: Some(3),
                limit: Some(2),
            },
        )
        .unwrap();
    let mut proposals_backward: ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            govmod,
            &QueryMsg::ReverseProposals {
                start_before: Some(6),
                limit: Some(2),
            },
        )
        .unwrap();

    let expected = ProposalResponse {
        id: 4,
        proposal: Proposal {
            title: "Text proposal 4.".to_string(),
            description: "This is a simple text proposal".to_string(),
            proposer: Addr::unchecked(CREATOR_ADDR),
            start_height: app.block_info().height,
            expiration: cw_utils::Expiration::AtHeight(app.block_info().height + 100),
            threshold: Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Majority {},
                quorum: PercentageThreshold::Percent(Decimal::percent(0)),
            },
            total_power: Uint128::new(100),
            msgs: vec![],
            status: Status::Open,
            votes: Votes::zero(),
            deposit_info: None,
        },
    };
    assert_eq!(proposals_forward.proposals[0], expected);
    assert_eq!(proposals_backward.proposals[1], expected);

    proposals_backward.proposals.reverse();
    assert_eq!(proposals_forward.proposals, proposals_backward.proposals);
}

#[test]
fn test_hooks() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        only_members_execute: false,
        deposit_info: None,
    };

    let governance_addr =
        instantiate_with_default_governance(&mut app, govmod_id, instantiate, None);
    let governance_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cw_core::msg::QueryMsg::GovernanceModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let dao = govmod_config.dao;

    // Expect no hooks
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::ProposalHooks {})
        .unwrap();
    assert_eq!(hooks.hooks.len(), 0);

    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::VoteHooks {})
        .unwrap();
    assert_eq!(hooks.hooks.len(), 0);

    let msg = ExecuteMsg::AddProposalHook {
        address: "some_addr".to_string(),
    };

    // Expect error as sender is not DAO
    let _err = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &msg,
            &[],
        )
        .unwrap_err();

    // Expect success as sender is now DAO
    let _res = app
        .execute_contract(dao.clone(), govmod_single.clone(), &msg, &[])
        .unwrap();

    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::ProposalHooks {})
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);

    // Expect error as hook is already set
    let _err = app
        .execute_contract(dao.clone(), govmod_single.clone(), &msg, &[])
        .unwrap_err();

    // Expect error as hook does not exist
    let _err = app
        .execute_contract(
            dao.clone(),
            govmod_single.clone(),
            &ExecuteMsg::RemoveProposalHook {
                address: "not_exist".to_string(),
            },
            &[],
        )
        .unwrap_err();

    let msg = ExecuteMsg::RemoveProposalHook {
        address: "some_addr".to_string(),
    };

    // Expect error as sender is not DAO
    let _err = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &msg,
            &[],
        )
        .unwrap_err();

    // Expect success
    let _res = app
        .execute_contract(dao.clone(), govmod_single.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::AddVoteHook {
        address: "some_addr".to_string(),
    };

    // Expect error as sender is not DAO
    let _err = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &msg,
            &[],
        )
        .unwrap_err();

    // Expect success as sender is now DAO
    let _res = app
        .execute_contract(dao.clone(), govmod_single.clone(), &msg, &[])
        .unwrap();

    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::VoteHooks {})
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);

    // Expect error as hook is already set
    let _err = app
        .execute_contract(dao.clone(), govmod_single.clone(), &msg, &[])
        .unwrap_err();

    // Expect error as hook does not exist
    let _err = app
        .execute_contract(
            dao.clone(),
            govmod_single.clone(),
            &ExecuteMsg::RemoveVoteHook {
                address: "not_exist".to_string(),
            },
            &[],
        )
        .unwrap_err();

    let msg = ExecuteMsg::RemoveVoteHook {
        address: "some_addr".to_string(),
    };

    // Expect error as sender is not DAO
    let _err = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &msg,
            &[],
        )
        .unwrap_err();

    // Expect success
    let _res = app.execute_contract(dao, govmod_single, &msg, &[]).unwrap();
}
