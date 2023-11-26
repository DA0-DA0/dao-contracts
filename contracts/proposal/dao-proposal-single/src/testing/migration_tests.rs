use cosmwasm_std::{to_json_binary, Addr, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, Executor};
use voting_v2::pre_propose::PreProposeInfo;
use dao_interface::query::{GetItemResponse, ProposalModuleCountResponse};
use dao_testing::contracts::{
    cw20_base_contract, cw20_stake_contract, cw20_staked_balances_voting_contract,
    dao_dao_contract, proposal_single_contract, v2_dao_dao_contract,
    v2_pre_propose_single_contract, v2_proposal_single_contract,
};
use dao_voting::{deposit::UncheckedDepositInfo, status::Status};
use dao_voting::pre_propose::ProposalCreationPolicy;
use crate::msg::QueryMsg;

use crate::testing::{
    execute::{execute_proposal, make_proposal, vote_on_proposal},
    instantiate::get_pre_propose_info,
    queries::{query_proposal, query_proposal_count},
};

/// This test attempts to simulate a realistic migration from DAO DAO
/// v2 to v3. Other tests in `/tests/tests.rs` check that versions and
/// top-level configs are updated correctly during migration. This
/// concerns itself more with more subtle state in the contracts that
/// is less functionality critical and thus more likely to be
/// overlooked in migration logic.
///
/// - I can migrate with tokens in the treasury and completed
///   proposals.
///
/// - I can migrate an open and unexecutable proposal, and use
///   `close_proposal_on_execution_failure` to close it once the
///   migration completes.
///
/// - Proposal count remains accurate after proposal migration.
///
/// - Items are not overriden during migration.
#[test]
fn test_v2_v3_full_migration() {
    let sender = Addr::unchecked("sender");

    let mut app = App::default();

    // ----
    // instantiate a v2 DAO
    // ----

    let proposal_code = app.store_code(v2_proposal_single_contract());
    let pre_proposal_code = app.store_code(v2_pre_propose_single_contract());
    let core_code = app.store_code(v2_dao_dao_contract());

    // cw20 staking and voting module has not changed across v2->v3 so
    // we use the current edition.
    let cw20_code = app.store_code(cw20_base_contract());
    let cw20_stake_code = app.store_code(cw20_stake_contract());
    let voting_code = app.store_code(cw20_staked_balances_voting_contract());

    let initial_balances = vec![Cw20Coin {
        address: sender.to_string(),
        amount: Uint128::new(2),
    }];

    let core = app
        .instantiate_contract(
            core_code,
            sender.clone(),
            &dao_interface_v2::msg::InstantiateMsg {
                admin: Some(sender.to_string()),
                name: "n".to_string(),
                description: "d".to_string(),
                image_url: Some("i".to_string()),
                automatically_add_cw20s: false,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: dao_interface_v2::state::ModuleInstantiateInfo {
                    code_id: voting_code,
                    msg: to_json_binary(&dao_voting_cw20_staked::msg::InstantiateMsg {
                        active_threshold: None,
                        token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                            code_id: cw20_code,
                            label: "token".to_string(),
                            name: "name".to_string(),
                            symbol: "symbol".to_string(),
                            decimals: 6,
                            initial_balances,
                            marketing: None,
                            staking_code_id: cw20_stake_code,
                            unstaking_duration: None,
                            initial_dao_balance: Some(Uint128::new(100)),
                        },
                    })
                    .unwrap(),
                    admin: Some(dao_interface_v2::state::Admin::CoreModule {}),
                    label: "voting".to_string(),
                    funds: vec![],
                },
                proposal_modules_instantiate_info: vec![
                    dao_interface_v2::state::ModuleInstantiateInfo {
                        code_id: proposal_code,
                        msg: to_json_binary(&dao_proposal_single_v2::msg::InstantiateMsg {
                            threshold: voting_v2::threshold::Threshold::AbsolutePercentage {
                                percentage: voting_v2::threshold::PercentageThreshold::Majority {},
                            },
                            max_voting_period: cw_utils::Duration::Height(6),
                            min_voting_period: None,
                            only_members_execute: false,
                            allow_revoting: false,
                            pre_propose_info:
                                // TODO use pre-propose module
                                // voting_v2::pre_propose::PreProposeInfo::ModuleMayPropose {
                                //     info: dao_interface_v2::state::ModuleInstantiateInfo {
                                //         code_id: pre_proposal_code,
                                //         msg: to_json_binary(
                                //             &dao_pre_propose_single_v2::InstantiateMsg {
                                //                 deposit_info: None,
                                //                 open_proposal_submission: false,
                                //                 extension: cosmwasm_std::Empty {},
                                //             },
                                //         )
                                //         .unwrap(),
                                //         admin: Some(dao_interface_v2::state::Admin::CoreModule {}),
                                //         funds: vec![],
                                //         label: "pre-propose module".to_string(),
                                //     },
                                // },
                                voting_v2::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                            close_proposal_on_execution_failure: true,
                        })
                        .unwrap(),
                        admin: Some(dao_interface_v2::state::Admin::CoreModule {}),
                        funds: vec![],
                        label: "proposal".to_string(),
                    },
                ],
                initial_items: Some(vec![dao_interface_v2::msg::InitialItem {
                    key: "key".to_string(),
                    value: "value".to_string(),
                }]),
                dao_uri: None,
            },
            &[],
            "core",
            Some(sender.to_string()),
        )
        .unwrap();
    app.execute(
        sender.clone(),
        WasmMsg::UpdateAdmin {
            contract_addr: core.to_string(),
            admin: core.to_string(),
        }
        .into(),
    )
    .unwrap();

    // ----
    // stake tokens in the DAO
    // ----

    let token = {
        let voting: Addr = app
            .wrap()
            .query_wasm_smart(&core, &dao_interface_v2::msg::QueryMsg::VotingModule {})
            .unwrap();
        let staking: Addr = app
            .wrap()
            .query_wasm_smart(
                &voting,
                &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
            )
            .unwrap();
        let token: Addr = app
            .wrap()
            .query_wasm_smart(
                &voting,
                &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
            )
            .unwrap();
        app.execute_contract(
            sender.clone(),
            token.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: staking.into_string(),
                amount: Uint128::new(1),
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
        app.update_block(next_block);
        token
    };

    // ----
    // create a proposal and add tokens to the treasury.
    // ----

    let proposal = {
        let modules: Vec<dao_interface_v2::state::ProposalModule> = app
            .wrap()
            .query_wasm_smart(
                &core,
                &dao_interface_v2::msg::QueryMsg::ProposalModules {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert!(modules.len() == 1);
        modules.into_iter().next().unwrap().address
    };
    // old config to assert against
    let config_v2: dao_proposal_single_v2::state::Config = app.wrap().query_wasm_smart(
        &proposal.to_string(),
        &QueryMsg::Config {},
    ).unwrap();

    app.execute_contract(
        core.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Propose(
            voting_v2::proposal::SingleChoiceProposeMsg {
                title: "t".to_string(),
                description: "d".to_string(),
                msgs: vec![WasmMsg::Execute {
                    contract_addr: core.to_string(),
                    msg: to_json_binary(&dao_interface_v2::msg::ExecuteMsg::UpdateCw20List {
                        to_add: vec![token.to_string()],
                        to_remove: vec![],
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
                proposer: None,
            },
        ),
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: voting_v2::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();
    let tokens: Vec<dao_interface_v2::query::Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            &core,
            &dao_interface_v2::msg::QueryMsg::Cw20Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        tokens,
        vec![dao_interface_v2::query::Cw20BalanceResponse {
            addr: token.clone(),
            balance: Uint128::new(100),
        }]
    );

    // ----
    // Create a proposal that is unexecutable without close_proposal_on_execution_failure
    // ----

    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Propose(
            voting_v2::proposal::SingleChoiceProposeMsg {
                title: "t".to_string(),
                description: "d".to_string(),
                msgs: vec![WasmMsg::Execute {
                    contract_addr: token.to_string(),
                    msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: sender.to_string(),
                        // more tokens than the DAO posseses.
                        amount: Uint128::new(101),
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
                proposer: None,
            },
        ),
        &[],
    )
    .unwrap();
    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Vote {
            proposal_id: 2,
            vote: voting_v2::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Execute { proposal_id: 2 },
        &[],
    )
    .unwrap();
    let dao_proposal_single_v2::query::ProposalResponse {
        proposal: dao_proposal_single_v2::proposal::SingleChoiceProposal { status, .. },
        ..
    } = app
        .wrap()
        .query_wasm_smart(
            &proposal,
            &dao_proposal_single_v2::msg::QueryMsg::Proposal { proposal_id: 2 },
        )
        .unwrap();
    assert_eq!(status, voting_v2::status::Status::ExecutionFailed {});

    // query existing proposals to assert against
    let proposals_v2: dao_proposal_single_v2::query::ProposalListResponse = app.wrap().query_wasm_smart(
        &proposal.clone(),
        &dao_proposal_single_v2::msg::QueryMsg::ListProposals {
            start_after: None,
            limit: None,
        }
    ).unwrap();

    // ----
    // create a proposal to migrate to v3
    // ----

    let v3_core_code = app.store_code(dao_dao_contract());
    let v3_proposal_code = app.store_code(proposal_single_contract());

    let pre_propose_info = get_pre_propose_info(
        &mut app,
        Some(UncheckedDepositInfo {
            denom: dao_voting::deposit::DepositToken::VotingModuleToken {},
            amount: Uint128::new(1),
            refund_policy: dao_voting::deposit::DepositRefundPolicy::OnlyPassed,
        }),
        false,
    );

    // TODO test migrate with timelock enabled
    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Propose(
            voting_v2::proposal::SingleChoiceProposeMsg {
                title: "t".to_string(),
                description: "d".to_string(),
                msgs: vec![
                    WasmMsg::Migrate {
                        contract_addr: core.to_string(),
                        new_code_id: v3_core_code,
                        msg: to_json_binary(&dao_interface::msg::MigrateMsg::FromCompatible {}).unwrap(),
                    }
                    .into(),
                    WasmMsg::Migrate {
                        contract_addr: proposal.to_string(),
                        new_code_id: v3_proposal_code,
                        msg: to_json_binary(&crate::msg::MigrateMsg::FromV2 { timelock: None }).unwrap(),
                    }
                    .into(),
                ],
                proposer: None,
            },
        ),
        &[],
    )
    .unwrap();
    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Vote {
            proposal_id: 3,
            vote: voting_v2::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender.clone(),
        proposal.clone(),
        &dao_proposal_single_v2::msg::ExecuteMsg::Execute { proposal_id: 3 },
        &[],
    )
    .unwrap();

    // ----
    // check that proposal count is still three after proposal state migration.
    // ----
    let count = query_proposal_count(&app, &proposal);
    assert_eq!(count, 3);

    // ----
    // check that proposal module counts have been updated.
    // ----
    let module_counts: ProposalModuleCountResponse = app
        .wrap()
        .query_wasm_smart(&core, &dao_interface::msg::QueryMsg::ProposalModuleCount {})
        .unwrap();
    assert_eq!(
        module_counts,
        ProposalModuleCountResponse {
            active_proposal_module_count: 1,
            total_proposal_module_count: 1,
        }
    );

    // ----
    // check that items are not overriden in migration.
    // ----
    let item: GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            &core,
            &dao_interface::msg::QueryMsg::GetItem {
                key: "key".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        item,
        GetItemResponse {
            item: Some("value".to_string())
        }
    );

    // ----
    // check that proposal can still be created an executed.
    // ----
    make_proposal(
        &mut app,
        &proposal,
        sender.as_str(),
        vec![WasmMsg::Execute {
            contract_addr: core.to_string(),
            msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateCw20List {
                to_add: vec![],
                to_remove: vec![token.into_string()],
            })
            .unwrap(),
            funds: vec![],
        }
        .into()],
    );
    vote_on_proposal(
        &mut app,
        &proposal,
        sender.as_str(),
        4,
        dao_voting::voting::Vote::Yes,
    );
    execute_proposal(&mut app, &proposal, sender.as_str(), 4);
    let tokens: Vec<dao_interface::query::Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            &core,
            &dao_interface::msg::QueryMsg::Cw20Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(tokens.is_empty());

    // query the config and assert fields are properly migrated
    let config: crate::state::Config = app.wrap().query_wasm_smart(
        &proposal,
        &QueryMsg::Config {},
    ).unwrap();
    assert_eq!(config.dao, config_v2.dao);
    assert_eq!(config.allow_revoting, config_v2.allow_revoting);
    assert_eq!(
        to_json_binary(&config.threshold).unwrap(),
        to_json_binary(&config_v2.threshold).unwrap(),
    );
    assert_eq!(config.close_proposal_on_execution_failure, config_v2.close_proposal_on_execution_failure);
    assert_eq!(config.max_voting_period, config_v2.max_voting_period);
    assert_eq!(config.min_voting_period, config_v2.min_voting_period);
    assert_eq!(config.only_members_execute, config_v2.only_members_execute);
    assert_eq!(config.timelock, None);

    // query migrated proposals
    let proposals_v3: crate::query::ProposalListResponse = app.wrap().query_wasm_smart(
        &proposal.to_string(),
        &crate::msg::QueryMsg::ListProposals {
            start_after: None,
            limit: None,
        }
    ).unwrap();

    // assert that all pre-migration props have been correctly migrated over
    for (i, prop_v2) in proposals_v2.proposals.iter().enumerate() {
        let migrated_prop = &proposals_v3.proposals[i];
        assert_eq!(prop_v2.id, migrated_prop.id);
        assert_eq!(prop_v2.proposal.title, migrated_prop.proposal.title);
        assert_eq!(prop_v2.proposal.description, migrated_prop.proposal.description);
        assert_eq!(prop_v2.proposal.proposer, migrated_prop.proposal.proposer);
        assert_eq!(prop_v2.proposal.start_height, migrated_prop.proposal.start_height);
        assert_eq!(prop_v2.proposal.min_voting_period, migrated_prop.proposal.min_voting_period);
        assert_eq!(prop_v2.proposal.expiration, migrated_prop.proposal.expiration);
        assert_eq!(
            to_json_binary(&prop_v2.proposal.threshold).unwrap(),
            to_json_binary(&migrated_prop.proposal.threshold).unwrap(),
        );
        assert_eq!(prop_v2.proposal.total_power, migrated_prop.proposal.total_power);
        assert_eq!(prop_v2.proposal.msgs, migrated_prop.proposal.msgs);
        assert_eq!(prop_v2.proposal.status.to_string(), migrated_prop.proposal.status.to_string());
        assert_eq!(prop_v2.proposal.allow_revoting, migrated_prop.proposal.allow_revoting);
        assert_eq!(None, migrated_prop.proposal.timelock);
    }
}
