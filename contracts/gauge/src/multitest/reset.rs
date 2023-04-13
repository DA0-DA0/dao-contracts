use cosmwasm_std::{Addr, Decimal, StdError, Uint128};
use cw_multi_test::{App, ContractWrapper, Executor};
use voting::Vote;

use crate::{
    msg::{GaugeMigrationConfig, GaugeResponse, ResetMigrationConfig, VoteInfo},
    multitest::suite::SuiteBuilder,
    ContractError,
};

const EPOCH: u64 = 7 * 86_400;
const RESET_EPOCH: u64 = 30 * 86_400;

#[test]
fn basic_gauge_reset() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let reward_to_distribute = (2000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 100)])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    let gauge_config = suite
        .instantiate_adapter_and_return_config(
            &[voter1, voter2],
            reward_to_distribute,
            None,
            RESET_EPOCH,
        )
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

    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    // voter1 was option voted for with two 100 voting powers combined
    assert_eq!(selected_set, vec![("voter1".to_owned(), Uint128::new(200))]);

    // cannot reset before epoch has passed
    assert_eq!(
        ContractError::ResetEpochNotPassed {},
        suite
            .reset_gauge("anyone", &gauge_contract, gauge_id, 10)
            .unwrap_err()
            .downcast()
            .unwrap()
    );

    // reset
    suite.advance_time(RESET_EPOCH);
    suite
        .reset_gauge("someone", &gauge_contract, gauge_id, 100) // 100 is way more than needed
        .unwrap();

    // check that gauge was reset
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(selected_set, vec![]);
    let votes = suite.query_list_votes(&gauge_contract, gauge_id).unwrap();
    assert_eq!(votes, vec![]);
    assert_eq!(
        suite.query_vote(&gauge_contract, gauge_id, voter1).unwrap(),
        None,
    );
    assert_eq!(
        suite.query_vote(&gauge_contract, gauge_id, voter2).unwrap(),
        None,
    );
    // options should still be there
    let options = suite.query_list_options(&gauge_contract, gauge_id).unwrap();
    assert_eq!(
        options,
        vec![
            ("voter1".to_owned(), Uint128::zero()),
            ("voter2".to_owned(), Uint128::zero())
        ]
    );

    // actually execute
    suite
        .execute_options(&gauge_contract, voter1, gauge_id)
        .unwrap();
    assert_eq!(
        suite
            .query_balance(suite.core.as_str(), reward_to_distribute.1)
            .unwrap(),
        reward_to_distribute.0,
        "nothing should be distributed yet, since all votes were reset"
    );

    // vote again
    suite
        .place_vote(
            &gauge_contract,
            voter1.to_owned(),
            gauge_id,
            Some(voter2.to_owned()),
        )
        .unwrap();

    // check that vote counts
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(selected_set, vec![("voter2".to_owned(), Uint128::new(100))]);

    // another epoch is fine
    suite.advance_time(EPOCH);
    suite
        .execute_options(&gauge_contract, voter1, gauge_id)
        .unwrap();

    assert_eq!(
        suite.query_balance(voter2, reward_to_distribute.1).unwrap(),
        2000u128
    );
}

fn store_old_gauge(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            gauge_orchestrator_1_6::contract::execute,
            gauge_orchestrator_1_6::contract::instantiate,
            gauge_orchestrator_1_6::contract::query,
        )
        .with_migrate(gauge_orchestrator_1_6::contract::migrate),
    );

    app.store_code(contract)
}

#[test]
fn gauge_migrate_with_reset() {
    let voter1 = "voter1";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100)])
        .build();

    // setup gauge
    suite.next_block();
    suite.propose_update_proposal_module(voter1, None).unwrap();
    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite.next_block();
    suite.execute_single_proposal(voter1, proposal).unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();
    assert_eq!(proposal_modules.len(), 2);
    let gauge_contract = proposal_modules[0].clone();

    // create adapter
    let gauge_adapter = suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &["option1", "option2"],
            (1000, "ujuno"),
            None,
            None,
        )
        .unwrap();

    // previous settings
    let response = suite.query_gauge(gauge_contract.clone(), 0).unwrap();
    assert_eq!(
        response,
        GaugeResponse {
            id: 0,
            title: "gauge".to_owned(),
            adapter: gauge_adapter.to_string(),
            epoch_size: EPOCH,
            min_percent_selected: Some(Decimal::percent(5)),
            max_options_selected: 10,
            max_available_percentage: None,
            is_stopped: false,
            next_epoch: suite.current_time() + 7 * 86400,
            reset: None,
        }
    );

    // now let's migrate the gauge and make sure nothing breaks
    let gauge_id = 0;
    // try to migrate to past reset should fail
    assert_eq!(
        ContractError::from(StdError::generic_err(
            "Next reset value cannot be earlier then current epoch!"
        )),
        suite
            .auto_migrate_gauge(
                &gauge_contract,
                vec![(
                    gauge_id,
                    GaugeMigrationConfig {
                        next_epoch: None,
                        reset: Some(ResetMigrationConfig {
                            reset_epoch: RESET_EPOCH,
                            next_reset: suite.current_time() - 1,
                        }),
                    },
                )],
            )
            .unwrap_err()
            .downcast()
            .unwrap()
    );

    // migrate to reset epoch
    suite
        .auto_migrate_gauge(
            &gauge_contract,
            vec![(
                gauge_id,
                GaugeMigrationConfig {
                    next_epoch: None,
                    reset: Some(ResetMigrationConfig {
                        reset_epoch: RESET_EPOCH,
                        next_reset: suite.current_time() + 100,
                    }),
                },
            )],
        )
        .unwrap();

    // check that gauge was migrated
    let response = suite.query_gauge(gauge_contract.clone(), 0).unwrap();
    assert_eq!(
        response,
        GaugeResponse {
            id: 0,
            title: "gauge".to_owned(),
            adapter: gauge_adapter.to_string(),
            epoch_size: EPOCH,
            min_percent_selected: Some(Decimal::percent(5)),
            max_options_selected: 10,
            max_available_percentage: None,
            is_stopped: false,
            next_epoch: suite.current_time() + 7 * 86400,
            reset: Some(crate::state::Reset {
                last: None,
                reset_each: RESET_EPOCH,
                next: suite.current_time() + 100,
            }),
        }
    );
}

#[test]
fn gauge_migrate_keeps_last_reset() {
    let voter1 = "voter1";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100)])
        .build();

    // setup gauge
    suite.next_block();
    suite.propose_update_proposal_module(voter1, None).unwrap();
    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite.next_block();
    suite.execute_single_proposal(voter1, proposal).unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();
    assert_eq!(proposal_modules.len(), 2);
    let gauge_contract = proposal_modules[0].clone();

    // create adapter
    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &["option1", "option2"],
            (1000, "ujuno"),
            None,
            Some(RESET_EPOCH),
        )
        .unwrap();
    let gauge_id = 0;

    // reset gauge once before migration
    suite.advance_time(RESET_EPOCH);
    suite
        .reset_gauge("someone", &gauge_contract, gauge_id, 1)
        .unwrap();
    let gauge = suite.query_gauge(gauge_contract.clone(), gauge_id).unwrap();
    assert_eq!(gauge.reset.unwrap().last, Some(suite.current_time()));

    // now let's migrate the gauge and make sure nothing breaks
    suite
        .auto_migrate_gauge(
            &gauge_contract,
            vec![(
                gauge_id,
                GaugeMigrationConfig {
                    next_epoch: None,
                    reset: Some(ResetMigrationConfig {
                        reset_epoch: RESET_EPOCH,
                        next_reset: suite.current_time() + 100,
                    }),
                },
            )],
        )
        .unwrap();

    // check that last reset is still the same
    let gauge = suite.query_gauge(gauge_contract.clone(), 0).unwrap();
    assert_eq!(gauge.reset.unwrap().last, Some(suite.current_time()));
}

#[test]
fn partial_reset() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let reward_to_distribute = (2000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 100)])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    let gauge_config = suite
        .instantiate_adapter_and_return_config(
            &[voter1, voter2],
            reward_to_distribute,
            None,
            RESET_EPOCH,
        )
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

    // vote for the gauge options
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
            Some(voter2.to_owned()),
        )
        .unwrap();

    // start resetting
    suite.advance_time(RESET_EPOCH);
    suite
        .reset_gauge("someone", &gauge_contract, gauge_id, 1)
        .unwrap();

    // try to vote during reset
    assert_eq!(
        ContractError::GaugeResetting(gauge_id),
        suite
            .place_vote(&gauge_contract, voter1, gauge_id, Some(voter2.to_owned()))
            .unwrap_err()
            .downcast()
            .unwrap()
    );
    // check selected set query
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(selected_set, vec![]);
    // check votes list
    let votes = suite.query_list_votes(&gauge_contract, gauge_id).unwrap();
    assert_eq!(votes, vec![]);

    // finish resetting
    suite
        .reset_gauge("someone", &gauge_contract, gauge_id, 1)
        .unwrap();
}

#[test]
fn vote_migration() {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 200)])
        .build();

    // setup old gauge version
    let old_gauge = store_old_gauge(&mut suite.app);
    let new_gauge = suite.gauge_code_id;
    suite.gauge_code_id = old_gauge;
    suite.next_block();
    suite.propose_update_proposal_module(voter1, None).unwrap();
    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, Vote::Yes)
        .unwrap();
    suite.next_block();
    suite.execute_single_proposal(voter1, proposal).unwrap();
    let proposal_modules = suite.query_proposal_modules().unwrap();
    assert_eq!(proposal_modules.len(), 2);
    let gauge_contract = proposal_modules[0].clone();
    let gauge_id = 0;

    // create adapter
    let option = suite
        .instantiate_adapter_and_return_config(&["option1", "option2"], (1000, "ujuno"), None, None)
        .unwrap();
    suite
        .app
        .execute_contract(
            Addr::unchecked(&suite.owner),
            gauge_contract.clone(),
            &gauge_orchestrator_1_6::msg::ExecuteMsg::CreateGauge(
                gauge_orchestrator_1_6::msg::GaugeConfig {
                    title: option.title,
                    adapter: option.adapter,
                    epoch_size: option.epoch_size,
                    min_percent_selected: option.min_percent_selected,
                    max_options_selected: option.max_options_selected,
                    max_available_percentage: option.max_available_percentage,
                },
            ),
            &[],
        )
        .unwrap();

    // place votes
    suite
        .place_vote(
            &gauge_contract,
            voter1,
            gauge_id,
            Some("option1".to_owned()),
        )
        .unwrap();
    suite
        .place_votes(
            &gauge_contract,
            voter2,
            gauge_id,
            vec![
                ("option1".to_owned(), Decimal::percent(50)),
                ("option2".to_owned(), Decimal::percent(50)),
            ],
        )
        .unwrap();

    // migrate the gauge to new code
    suite.gauge_code_id = new_gauge;
    suite
        .auto_migrate_gauge(
            &gauge_contract,
            vec![(
                gauge_id,
                GaugeMigrationConfig {
                    next_epoch: None,
                    reset: Some(ResetMigrationConfig {
                        reset_epoch: RESET_EPOCH,
                        next_reset: suite.current_time() + 100,
                    }),
                },
            )],
        )
        .unwrap();

    // check that all votes are still there
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(
        selected_set,
        vec![
            ("option1".to_owned(), Uint128::new(200)),
            ("option2".to_owned(), Uint128::new(100))
        ]
    );
    let votes = suite.query_list_votes(&gauge_contract, gauge_id).unwrap();
    let expected_votes1 = VoteInfo {
        voter: voter1.to_owned(),
        votes: vec![crate::state::Vote {
            option: "option1".to_owned(),
            weight: Decimal::one(),
        }],
        cast: None,
    };
    let expected_votes2 = VoteInfo {
        voter: voter2.to_owned(),
        votes: vec![
            crate::state::Vote {
                option: "option1".to_owned(),
                weight: Decimal::percent(50),
            },
            crate::state::Vote {
                option: "option2".to_owned(),
                weight: Decimal::percent(50),
            },
        ],
        cast: None,
    };
    assert_eq!(
        votes,
        vec![expected_votes1.clone(), expected_votes2.clone()]
    );
    let vote1 = suite.query_vote(&gauge_contract, gauge_id, voter1).unwrap();
    assert_eq!(vote1, Some(expected_votes1));
    let vote2 = suite.query_vote(&gauge_contract, gauge_id, voter2).unwrap();
    assert_eq!(vote2, Some(expected_votes2));

    // change vote
    suite
        .place_vote(
            &gauge_contract,
            voter1,
            gauge_id,
            Some("option2".to_owned()),
        )
        .unwrap();
    let vote1 = suite.query_vote(&gauge_contract, gauge_id, voter1).unwrap();
    assert_eq!(
        vote1,
        Some(VoteInfo {
            voter: voter1.to_owned(),
            votes: vec![crate::state::Vote {
                option: "option2".to_owned(),
                weight: Decimal::one()
            }],
            cast: Some(suite.current_time()),
        })
    );

    // reset the gauge
    suite.advance_time(100); // only 100 seconds, because we set it in the migration
    suite
        .reset_gauge("someone", &gauge_contract, gauge_id, 10)
        .unwrap();

    // check that all votes are gone
    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(selected_set, vec![]);
    let votes = suite.query_list_votes(&gauge_contract, gauge_id).unwrap();
    assert_eq!(votes, vec![]);
    let vote1 = suite.query_vote(&gauge_contract, gauge_id, voter1).unwrap();
    assert_eq!(vote1, None);
    let vote2 = suite.query_vote(&gauge_contract, gauge_id, voter2).unwrap();
    assert_eq!(vote2, None);
}
