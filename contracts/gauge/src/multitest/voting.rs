use cosmwasm_std::{Decimal, Uint128};
use voting::Vote;

use super::suite::SuiteBuilder;
use crate::error::ContractError;
use crate::msg::VoteInfo;

const EPOCH: u64 = 7 * 86_400;

#[test]
fn add_option() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 200)])
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.to_string(), None)
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.to_string(), proposal)
        .unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();

    let gauge_contract = proposal_modules[0].clone();

    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &[voter1, voter2],
            (1000, "ujuno"),
            None,
        )
        .unwrap();

    let gauge_id = 0; // first created gauge

    // gauge returns list all options; it does query adapter at initialization
    let options = suite.query_list_options(&gauge_contract, gauge_id).unwrap();
    assert_eq!(options.len(), 2);

    // Voting members can add options
    suite
        .add_option(&gauge_contract, voter1, gauge_id, "addedoption1")
        .unwrap();
    suite
        .add_option(&gauge_contract, voter2, gauge_id, "addedoption2")
        .unwrap();
    let options = suite.query_list_options(&gauge_contract, gauge_id).unwrap();
    // added options are automatically voted for by creators
    assert_eq!(
        options,
        vec![
            ("addedoption1".to_owned(), Uint128::zero()),
            ("addedoption2".to_owned(), Uint128::zero()),
            ("voter1".to_owned(), Uint128::zero()),
            ("voter2".to_owned(), Uint128::zero())
        ]
    );

    // Non-voting members cannot add options
    let err = suite
        .add_option(&gauge_contract, "random_voter", gauge_id, "addedoption3")
        .unwrap_err();
    assert_eq!(
        ContractError::NoVotingPower("random_voter".to_owned()),
        err.downcast().unwrap()
    );
}

fn simple_vote(voter: &str, option: &str, percentage: u64) -> VoteInfo {
    VoteInfo {
        voter: voter.to_string(),
        votes: vec![crate::state::Vote {
            option: option.to_string(),
            weight: Decimal::percent(percentage),
        }],
    }
}

fn multi_vote(voter: &str, votes: &[(&str, u64)]) -> VoteInfo {
    let votes = votes
        .iter()
        .map(|(opt, percentage)| crate::state::Vote {
            option: opt.to_string(),
            weight: Decimal::percent(*percentage),
        })
        .collect();
    VoteInfo {
        voter: voter.to_string(),
        votes,
    }
}

#[test]
fn vote_for_option() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 200)])
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.to_string(), None)
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.to_string(), proposal)
        .unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();

    let gauge_contract = proposal_modules[0].clone();

    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &[voter1, voter2],
            (1000, "ujuno"),
            None,
        )
        .unwrap();

    let gauge_id = 0; // first created gauge

    // vote for option from adapter (voting members are by default
    // options in adapter in this test suite)
    suite
        .place_votes(
            &gauge_contract,
            voter1.to_owned(),
            gauge_id,
            Some(vec![(voter1.to_owned(), Decimal::percent(90))]),
        )
        .unwrap();
    assert_eq!(
        simple_vote(voter1, voter1, 90),
        suite
            .query_vote(&gauge_contract, gauge_id, voter1)
            .unwrap()
            .unwrap(),
    );
    // check tally is proper
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(selected_set, vec![(voter1.to_string(), Uint128::new(90))]);

    // change vote for option added through gauge
    suite
        .add_option(&gauge_contract, voter1, gauge_id, "option1")
        .unwrap();
    suite
        .add_option(&gauge_contract, voter1, gauge_id, "option2")
        .unwrap();
    // voter2 drops vote as well
    suite
        .place_votes(
            &gauge_contract,
            voter2.to_owned(),
            gauge_id,
            Some(vec![
                ("option1".to_owned(), Decimal::percent(50)),
                ("option2".to_owned(), Decimal::percent(50)),
            ]),
        )
        .unwrap();
    assert_eq!(
        vec![
            simple_vote(voter1, voter1, 90),
            multi_vote(voter2, &[("option1", 50), ("option2", 50)]),
        ],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );

    // placing vote again overwrites previous ones
    suite
        .place_votes(
            &gauge_contract,
            voter1.to_owned(),
            gauge_id,
            Some(vec![("option1".to_owned(), Decimal::percent(90))]),
        )
        .unwrap();
    suite
        .place_votes(
            &gauge_contract,
            voter2.to_owned(),
            gauge_id,
            Some(vec![("option1".to_owned(), Decimal::percent(90))]),
        )
        .unwrap();
    assert_eq!(
        vec![
            simple_vote(voter1, "option1", 90),
            simple_vote(voter2, "option1", 90),
        ],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );

    // vote for non-existing option
    let err = suite
        .place_vote(
            &gauge_contract,
            voter1.to_owned(),
            gauge_id,
            Some("random option".to_owned()),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::OptionDoesNotExists {
            option: "random option".to_owned(),
            gauge_id
        },
        err.downcast().unwrap()
    );
}

#[test]
fn remove_vote() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 200)])
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.to_string(), None)
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.to_string(), proposal)
        .unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();

    let gauge_contract = proposal_modules[0].clone();

    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &[voter1, voter2],
            (1000, "ujuno"),
            None,
        )
        .unwrap();

    let gauge_id = 0; // first created gauge

    // vote for option from adapter (voting members are by default
    // options in adapter in this test suite)
    suite
        .place_vote(
            &gauge_contract,
            voter1.to_owned(),
            gauge_id,
            Some(voter1.to_owned()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter2.to_owned(),
            gauge_id,
            Some(voter1.to_owned()),
        )
        .unwrap();
    assert_eq!(
        vec![
            simple_vote(voter1, voter1, 100),
            simple_vote(voter2, voter1, 100),
        ],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );

    // remove vote
    suite
        .place_vote(&gauge_contract, voter1.to_owned(), gauge_id, None)
        .unwrap();
    assert_eq!(
        vec![simple_vote(voter2, voter1, 100)],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );
    assert_eq!(
        suite.query_vote(&gauge_contract, gauge_id, voter1).unwrap(),
        None
    );
    assert_eq!(
        suite.query_vote(&gauge_contract, gauge_id, voter2).unwrap(),
        Some(simple_vote(voter2, voter1, 100)),
    );

    // remove nonexisting vote
    let err = suite
        .place_vote(&gauge_contract, voter1.to_owned(), gauge_id, None)
        .unwrap_err();
    assert_eq!(
        ContractError::CannotRemoveNonexistingVote {},
        err.downcast().unwrap()
    );
}

#[test]
fn votes_stays_the_same_after_execution() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let reward_to_distribute = (1000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 100)])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    let gauge_config = suite
        .instantiate_adapter_and_return_config(&[voter1, voter2], reward_to_distribute, None)
        .unwrap();
    suite
        .propose_update_proposal_module(voter1.to_string(), vec![gauge_config])
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.to_string(), proposal)
        .unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();
    let gauge_contract = proposal_modules[0].clone();

    let gauge_id = 0;

    // vote for one of the options in gauge
    suite
        .place_vote(
            &gauge_contract,
            voter1.to_owned(),
            gauge_id,
            Some(voter1.to_owned()), // option to vote for
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter2.to_owned(),
            gauge_id,
            Some(voter1.to_owned()),
        )
        .unwrap();

    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    // voter1 was option voted for with two 100 voting powers combined
    assert_eq!(selected_set, vec![("voter1".to_owned(), Uint128::new(200))]);

    // before advancing specified epoch tally won't get sampled
    suite.advance_time(EPOCH);

    assert_eq!(
        vec![
            simple_vote(voter1, voter1, 100),
            simple_vote(voter2, voter1, 100)
        ],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );
    suite
        .execute_options(&gauge_contract, voter1, gauge_id)
        .unwrap();

    assert_eq!(
        vec![
            simple_vote(voter1, voter1, 100),
            simple_vote(voter2, voter1, 100)
        ],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );
    assert_eq!(
        suite.query_vote(&gauge_contract, gauge_id, voter1).unwrap(),
        Some(simple_vote(voter1, voter1, 100)),
    );
    assert_eq!(
        suite.query_vote(&gauge_contract, gauge_id, voter2).unwrap(),
        Some(simple_vote(voter2, voter1, 100)),
    );
}

#[test]
fn vote_for_max_capped_option() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 100)])
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.to_string(), None)
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.to_string(), proposal)
        .unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();

    let gauge_contract = proposal_modules[0].clone();

    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &[voter1, voter2],
            (1000, "ujuno"),
            Some(Decimal::percent(10)),
        )
        .unwrap();

    let gauge_id = 0; // first created gauge

    // wait until epoch passes
    suite.advance_time(EPOCH);

    // change vote for option added through gauge
    suite
        .add_option(&gauge_contract, voter1, gauge_id, "option1")
        .unwrap();
    suite
        .add_option(&gauge_contract, voter1, gauge_id, "option2")
        .unwrap();

    // vote 100% voting power on 'voter1' option (100 weight)
    suite
        .place_vote(
            &gauge_contract,
            voter1,
            gauge_id,
            Some("option1".to_owned()),
        )
        .unwrap();
    // vote 10% voting power on 'voter2' option (10 weight)
    suite
        .place_votes(
            &gauge_contract,
            voter2,
            gauge_id,
            vec![("option2".to_owned(), Decimal::percent(10))],
        )
        .unwrap();

    assert_eq!(
        vec![
            multi_vote(voter1, &[("option1", 100)]),
            multi_vote(voter2, &[("option2", 10)]),
        ],
        suite.query_list_votes(&gauge_contract, gauge_id).unwrap()
    );

    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    // Despite 'option1' having 100 voting power and option2 having 10 voting power,
    // because of max vote cap set to 10% now 'option1' will have its power decreased to 10% * 110
    // 'option2' stays at 10 voting power as it was below 10% of total votes
    assert_eq!(
        selected_set,
        vec![
            ("option1".to_owned(), Uint128::new(11)),
            ("option2".to_owned(), Uint128::new(10))
        ]
    );
}
