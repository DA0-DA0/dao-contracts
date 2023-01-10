use cosmwasm_std::{Decimal, Uint128};
use voting::Vote;

use super::suite::SuiteBuilder;

use crate::error::ContractError;
use crate::msg::GaugeResponse;

const EPOCH: u64 = 7 * 86_400;

#[test]
fn create_gauge() {
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

    // Second proposal module is cw proposal single, first one is newly added gauge
    assert_eq!(proposal_modules.len(), 2);
    let gauge_contract = proposal_modules[0].clone();

    let gauge_adapter = suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &[voter1, voter2],
            (1000, "ujuno"),
        )
        .unwrap();

    let response = suite.query_gauge(gauge_contract, 0).unwrap();
    assert_eq!(
        response,
        GaugeResponse {
            id: 0,
            title: "gauge".to_owned(),
            adapter: gauge_adapter.to_string(),
            epoch_size: EPOCH,
            min_percent_selected: Some(Decimal::percent(5)),
            max_options_selected: 10,
            is_stopped: false,
            next_epoch: suite.current_time() + 7 * 86400,
        }
    );
}

/// attach adaptor in instantiate
#[test]
fn execute_gauge() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let reward_to_distribute = (1000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 100)])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    let gauge_config = suite
        .instantiate_adapter_and_return_config(&[voter1, voter2], reward_to_distribute)
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

    suite
        .execute_options(&gauge_contract, voter1, gauge_id)
        .unwrap();

    assert_eq!(
        suite.query_balance(voter1, reward_to_distribute.1).unwrap(),
        1000u128
    );
}

#[test]
fn execute_stopped_gauge() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let reward_to_distribute = (1000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 100)])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    let gauge_config = suite
        .instantiate_adapter_and_return_config(&[voter1, voter2], reward_to_distribute)
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

    // stop the gauge by not-owner
    let err = suite
        .stop_gauge(&gauge_contract, voter1, gauge_id)
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

    // stop the gauge by owner
    suite
        .stop_gauge(&gauge_contract, suite.owner.clone(), gauge_id)
        .unwrap();

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

    // Despite gauge being stopped, user
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(selected_set, vec![("voter1".to_owned(), Uint128::new(200))]);

    // before advancing specified epoch tally won't get sampled
    suite.advance_time(EPOCH);

    let err = suite
        .execute_options(&gauge_contract, voter1, gauge_id)
        .unwrap_err();
    assert_eq!(
        ContractError::GaugeStopped(gauge_id),
        err.downcast().unwrap()
    );
}
