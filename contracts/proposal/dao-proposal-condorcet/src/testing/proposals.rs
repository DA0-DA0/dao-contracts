use cosmwasm_std::{to_json_binary, WasmMsg};
use cw_utils::Duration;

use crate::{
    config::UncheckedConfig,
    msg::ExecuteMsg,
    proposal::{ProposalResponse, Status},
    tally::Winner,
    testing::suite::unimportant_message,
    ContractError,
};

use super::{is_error, suite::SuiteBuilder};

// a condorcet winner does not exist and the proposal is closed.
#[test]
fn test_proposal_lifecycle_closed() {
    let mut suite = SuiteBuilder::default()
        .with_voters(&[
            ("blue", 10),
            ("violet", 10),
            ("magenta", 10),
            ("gold", 10),
            ("crimson", 10),
            ("turquoise", 10),
        ])
        .with_proposal(2)
        .build();

    suite.vote("blue", 1, vec![0, 2, 1]).unwrap();
    suite.vote("violet", 1, vec![1, 0, 2]).unwrap();
    suite.vote("magenta", 1, vec![2, 1, 0]).unwrap();
    suite.vote("gold", 1, vec![1, 0, 2]).unwrap();
    suite.vote("crimson", 1, vec![0, 2, 1]).unwrap();
    suite.vote("turquoise", 1, vec![2, 0, 1]).unwrap();

    suite.a_day_passes();

    let (winner, status) = suite.query_winner_and_status(1);
    assert_eq!(winner, Winner::Never);
    assert_eq!(status, Status::Rejected);

    suite.close("crimson", 1).unwrap();

    let (_, status) = suite.query_winner_and_status(1);
    assert_eq!(status, Status::Closed);
}

#[test]
fn test_make_proposal() {
    let mut suite = SuiteBuilder::default().build();
    let id = suite
        .propose(suite.sender(), vec![vec![unimportant_message()]])
        .unwrap();
    let ProposalResponse { proposal, tally } = suite.query_proposal(id);

    assert_eq!(proposal.id, id);
    assert_eq!(proposal.choices.len(), 2);
    assert_eq!(proposal.choices[0].msgs[0], unimportant_message());
    assert_eq!(proposal.choices[1].msgs, vec![]); // none-of-the-above added to the end.

    assert_eq!(tally.candidates(), 2);
    assert_eq!(tally.winner, Winner::None);
    assert_eq!(tally.power_outstanding, proposal.total_power);
    assert_eq!(tally.start_height, suite.block_height());
}

#[test]
fn test_proposal_zero_choices() {
    let mut suite = SuiteBuilder::default().build();
    let err = suite.propose(suite.sender(), vec![]);
    is_error!(err, &ContractError::ZeroChoices {}.to_string());
}

#[test]
fn test_no_propose_zero_voting_power() {
    let mut suite = SuiteBuilder::default().build();
    let err = suite.propose("someone", vec![]);
    is_error!(err, &ContractError::ZeroVotingPower {}.to_string());
}

#[test]
fn test_proposal_lifeclyle_execution_failed() {
    let mut suite = SuiteBuilder::default().with_proposal(1).build();

    suite.vote(suite.sender(), 1, vec![0, 1]).unwrap();

    let (winner, status) = suite.query_winner_and_status(1);
    assert_eq!(winner, Winner::Undisputed(0));
    assert_eq!(status, Status::Open); // min voting period!

    suite.a_day_passes();

    let (_, status) = suite.query_winner_and_status(1);
    assert_eq!(status, Status::Passed { winner: 0 });

    suite.execute(suite.sender(), 1).unwrap();

    let (winner, status) = suite.query_winner_and_status(1);
    assert_eq!(status, Status::ExecutionFailed);
    assert_eq!(winner, Winner::Undisputed(0));
}

#[test]
fn test_proposal_never_reaches_quorum() {
    let mut suite = SuiteBuilder::default()
        .with_voters(&[("pleb", 1), ("belp", 10)])
        .with_proposal(2)
        .build();

    suite.vote("pleb", 1, vec![0, 2, 1]).unwrap();

    // seven days pass
    suite.a_week_passes();

    let (winner, status) = suite.query_winner_and_status(1);
    assert_eq!(winner, Winner::Some(0));
    assert_eq!(status, Status::Rejected);
}

#[test]
fn test_proposal_passes_after_expiry() {
    let mut suite = SuiteBuilder::default()
        .with_voters(&[("pleb", 15), ("belp", 85)])
        .with_proposal(2)
        .build();

    suite.vote("pleb", 1, vec![0, 2, 1]).unwrap();

    suite.a_week_passes();

    let (winner, status) = suite.query_winner_and_status(1);
    assert_eq!(winner, Winner::Some(0));
    assert_eq!(status, Status::Passed { winner: 0 });
}

#[test]
fn test_no_vote_after_expiry() {
    let mut suite = SuiteBuilder::default().with_proposal(1).build();

    suite.a_week_passes();

    let err = suite.vote(suite.sender(), 1, vec![0, 1]);
    is_error!(err, &ContractError::Expired {}.to_string());
}

#[test]
fn test_no_revoting() {
    let mut suite = SuiteBuilder::default().with_proposal(1).build();

    suite.vote(suite.sender(), 1, vec![0, 1]).unwrap();

    let err = suite.vote(suite.sender(), 1, vec![0, 1]);
    is_error!(err, &ContractError::Voted {}.to_string());
}

#[test]
fn test_no_vote_zero_power() {
    let mut suite = SuiteBuilder::default().with_proposal(1).build();
    let err = suite.vote("somebody", 1, vec![0, 1]);
    is_error!(err, &ContractError::ZeroVotingPower {}.to_string());
}

#[test]
fn test_proposal_set_config() {
    let mut suite = SuiteBuilder::default().build();
    let config = suite.query_config();

    suite
        .propose(
            suite.sender(),
            vec![vec![WasmMsg::Execute {
                contract_addr: suite.condorcet.to_string(),
                msg: to_json_binary(&ExecuteMsg::SetConfig(UncheckedConfig {
                    quorum: config.quorum,
                    voting_period: config.voting_period,
                    min_voting_period: None,
                    close_proposals_on_execution_failure: false,
                }))
                .unwrap(),
                funds: vec![],
            }
            .into()]],
        )
        .unwrap();
    // before passing the earlier one make another proposal who's
    // execution will fail if configs are correctly checked in
    // set_config. this proposal failing and entering the
    // ExecutionFailed state will indicate that configs are being
    // validated and that close_proposal_on_execution_failure is being
    // applied on a per-proposal basis.
    suite
        .propose(
            suite.sender(),
            vec![vec![WasmMsg::Execute {
                contract_addr: suite.condorcet.to_string(),
                msg: to_json_binary(&ExecuteMsg::SetConfig(UncheckedConfig {
                    quorum: config.quorum,
                    voting_period: config.voting_period,
                    min_voting_period: Some(Duration::Height(10)),
                    close_proposals_on_execution_failure: false,
                }))
                .unwrap(),
                funds: vec![],
            }
            .into()]],
        )
        .unwrap();

    suite.a_day_passes();

    suite.vote(suite.sender(), 1, vec![0, 1]).unwrap();
    suite.execute(suite.sender(), 1).unwrap();

    let new_config = suite.query_config();
    assert_eq!(new_config.quorum, config.quorum);
    assert_eq!(new_config.voting_period, config.voting_period);
    assert_eq!(new_config.min_voting_period, None);
    assert!(!new_config.close_proposals_on_execution_failure);

    suite.vote(suite.sender(), 2, vec![0, 1]).unwrap();
    suite.execute(suite.sender(), 2).unwrap();

    let (_, status) = suite.query_winner_and_status(2);
    assert_eq!(status, Status::ExecutionFailed);
}

#[test]
fn test_execution_fail_handling() {
    let mut suite = SuiteBuilder::default().with_proposal(1);
    suite.instantiate.close_proposals_on_execution_failure = false;
    let mut suite = suite.build();

    suite.vote(suite.sender(), 1, vec![0, 1]).unwrap();
    // important that this errors the whole transaction to ensure that
    // no state changes get committed.
    suite.execute(suite.sender(), 1).unwrap_err();
}
