use cosmwasm_std::{to_binary, Addr, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, AppResponse, Executor};
use dao_interface::{Admin, ModuleInstantiateInfo};
use dao_testing::contracts::{
    dao_core_contract, dao_voting_cw4_contract, proposal_single_contract,
};

use crate::{
    testing::helpers::dao_voting_cw20_staked_contract,
    types::{MigrationParams, V1CodeIds, V2CodeIds},
};

use super::helpers::{migrator_contract, InitDaoDataV1, SENDER_ADDR};

/// Instantiate a basic DAO with proposal and voting modules.
pub fn init_dao_v1(mut app: App, data: Option<InitDaoDataV1>) -> (App, Addr, Addr, V1CodeIds) {
    let data = data.unwrap_or_default();
    let sender = Addr::unchecked(SENDER_ADDR);

    // Store v1 codes
    let core_code = app.store_code(data.core_code);
    let proposal_code = app.store_code(data.proposal_code);
    let cw20_code = app.store_code(data.cw20_code);
    let cw20_stake_code = app.store_code(data.cw20_stake_code);
    let cw20_voting_code = app.store_code(data.cw20_voting_code);

    let v1_code_ids = V1CodeIds {
        proposal_single: proposal_code,
        cw4_voting: 9999,
        cw20_stake: cw20_stake_code,
        cw20_staked_balances_voting: cw20_voting_code,
    };

    let initial_balances = vec![Cw20Coin {
        address: SENDER_ADDR.to_string(),
        amount: Uint128::new(2),
    }];

    let core_addr = app
        .instantiate_contract(
            core_code,
            sender.clone(),
            &cw_core_v1::msg::InstantiateMsg {
                admin: Some(SENDER_ADDR.to_string()),
                name: "n".to_string(),
                description: "d".to_string(),
                image_url: Some("i".to_string()),
                automatically_add_cw20s: false,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: cw_core_v1::msg::ModuleInstantiateInfo {
                    code_id: cw20_voting_code,
                    msg: to_binary(&dao_voting_cw20_staked::msg::InstantiateMsg {
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
                    admin: cw_core_v1::msg::Admin::CoreContract {},
                    label: "voting".to_string(),
                },
                proposal_modules_instantiate_info: vec![cw_core_v1::msg::ModuleInstantiateInfo {
                    code_id: proposal_code,
                    msg: to_binary(&cw_proposal_single_v1::msg::InstantiateMsg {
                        threshold: voting_v1::Threshold::AbsolutePercentage {
                            percentage: voting_v1::PercentageThreshold::Majority {},
                        },
                        max_voting_period: cw_utils_v1::Duration::Height(6),
                        min_voting_period: None,
                        only_members_execute: false,
                        allow_revoting: false,
                        deposit_info: None,
                    })
                    .unwrap(),
                    admin: cw_core_v1::msg::Admin::CoreContract {},
                    label: "proposal".to_string(),
                }],
                initial_items: Some(vec![cw_core_v1::msg::InitialItem {
                    key: "key".to_string(),
                    value: "value".to_string(),
                }]),
            },
            &[],
            "core",
            Some(sender.to_string()),
        )
        .unwrap();

    app.execute(
        sender.clone(),
        WasmMsg::UpdateAdmin {
            contract_addr: core_addr.to_string(),
            admin: core_addr.to_string(),
        }
        .into(),
    )
    .unwrap();

    // Get modules addrs
    let proposal_addr = {
        let modules: Vec<Addr> = app
            .wrap()
            .query_wasm_smart(
                &core_addr,
                &cw_core_v1::msg::QueryMsg::ProposalModules {
                    start_at: None,
                    limit: None,
                },
            )
            .unwrap();
        assert!(modules.len() == 1);
        modules.into_iter().next().unwrap()
    };
    let voting_addr: Addr = app
        .wrap()
        .query_wasm_smart(&core_addr, &cw_core_v1::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            &voting_addr,
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            &voting_addr,
            &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();

    // Stake token
    app.execute_contract(
        sender.clone(),
        token_addr.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: Uint128::new(1),
            msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
        },
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // ----
    // create a proposal and add tokens to the treasury.
    // ----

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Propose {
            title: "t".to_string(),
            description: "d".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_binary(&cw_core_v1::msg::ExecuteMsg::UpdateCw20List {
                    to_add: vec![token_addr.to_string()],
                    to_remove: vec![],
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: voting_v1::Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender,
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let tokens: Vec<cw_core_v1::query::Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            &core_addr,
            &cw_core_v1::msg::QueryMsg::Cw20Balances {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        tokens,
        vec![cw_core_v1::query::Cw20BalanceResponse {
            addr: token_addr,
            balance: Uint128::new(100),
        }]
    );

    (app, core_addr, proposal_addr, v1_code_ids)
}

fn execute_migration(
    mut app: App,
    core_addr: Addr,
    proposal_addr: Addr,
    v1_code_ids: V1CodeIds,
) -> Result<AppResponse, anyhow::Error> {
    let sender = Addr::unchecked(SENDER_ADDR);
    let migrator_code_id = app.store_code(migrator_contract());
    let v2_core_code_id = app.store_code(dao_core_contract());
    let v2_proposal_code = app.store_code(proposal_single_contract());
    let v2_cw4_voting = app.store_code(dao_voting_cw4_contract());
    let v2_cw20_voting = app.store_code(dao_voting_cw20_staked_contract());

    println!("contract id: {:?}", v2_proposal_code);
    let v2_code_ids = V2CodeIds {
        proposal_single: v2_proposal_code,
        cw4_voting: v2_cw4_voting,
        cw20_stake: v1_code_ids.cw20_stake,
        cw20_staked_balances_voting: v2_cw20_voting,
    };

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Propose {
            title: "t2".to_string(),
            description: "d2".to_string(),
            msgs: vec![
                WasmMsg::Migrate {
                    contract_addr: core_addr.to_string(),
                    new_code_id: v2_core_code_id,
                    msg: to_binary(&dao_core::msg::MigrateMsg::FromV1 { dao_uri: None }).unwrap(),
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: core_addr.to_string(),
                    msg: to_binary(&dao_core::msg::ExecuteMsg::UpdateProposalModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: migrator_code_id,
                            msg: to_binary(&crate::msg::InstantiateMsg {
                                sub_daos: None,
                                migration_params: MigrationParams {
                                    migrate_stake_cw20_manager: Some(true),
                                    close_proposal_on_execution_failure: true,
                                    pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                                },
                                v1_code_ids,
                                v2_code_ids,
                            })
                            .unwrap(),
                            admin: Some(Admin::CoreModule {}),
                            label: "migrator".to_string(),
                        }],
                        to_disable: vec![],
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            ],
        },
        &[],
    ).unwrap();

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Vote {
            proposal_id: 2,
            vote: voting_v1::Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender,
        proposal_addr,
        &cw_proposal_single_v1::msg::ExecuteMsg::Execute { proposal_id: 2 },
        &[],
    )
}

#[test]
fn test_migration_v1_v2() {
    let app = App::default();

    // ----
    // instantiate a v1 DAO
    // ----
    let (app, core_addr, proposal_addr, v1_code_ids) = init_dao_v1(app, None);

    let res = execute_migration(app, core_addr, proposal_addr, v1_code_ids).unwrap();
    println!("{:?}", res)
}
