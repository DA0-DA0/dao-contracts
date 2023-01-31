use std::borrow::BorrowMut;

use cosmwasm_std::{to_binary, Addr, WasmMsg};
use cw_multi_test::{next_block, App, AppResponse, Executor};
use dao_interface::{Admin, ModuleInstantiateInfo};

use crate::{
    testing::helpers::get_module_addrs,
    types::{MigrationParams, V1CodeIds},
};

use super::helpers::{
    get_cw20_init_msg, get_cw4_init_msg, get_v1_code_ids, get_v2_code_ids, migrator_contract,
    set_cw20_to_dao, set_dummy_proposal, ExecuteParams, ModuleAddrs, VotingType, SENDER_ADDR,
};

pub fn init_v1(app: &mut App, sender: Addr, voting_type: VotingType) -> (Addr, V1CodeIds) {
    let (code_ids, v1_code_ids) = get_v1_code_ids(app);

    let (voting_code_id, msg) = match voting_type {
        VotingType::Cw4 => (
            code_ids.cw4_voting,
            to_binary(&get_cw4_init_msg(code_ids.clone())).unwrap(),
        ),
        VotingType::Cw20 => (
            code_ids.cw20_voting,
            to_binary(&get_cw20_init_msg(code_ids.clone())).unwrap(),
        ),
    };

    let core_addr = app
        .instantiate_contract(
            code_ids.core,
            sender.clone(),
            &cw_core_v1::msg::InstantiateMsg {
                admin: Some(SENDER_ADDR.to_string()),
                name: "n".to_string(),
                description: "d".to_string(),
                image_url: Some("i".to_string()),
                automatically_add_cw20s: false,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: cw_core_v1::msg::ModuleInstantiateInfo {
                    code_id: voting_code_id,
                    msg,
                    admin: cw_core_v1::msg::Admin::CoreContract {},
                    label: "voting".to_string(),
                },
                proposal_modules_instantiate_info: vec![cw_core_v1::msg::ModuleInstantiateInfo {
                    code_id: code_ids.proposal_single,
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

    app.update_block(next_block);

    app.execute(
        sender,
        WasmMsg::UpdateAdmin {
            contract_addr: core_addr.to_string(),
            admin: core_addr.to_string(),
        }
        .into(),
    )
    .unwrap();

    (core_addr, v1_code_ids)
}

/// Instantiate a basic DAO with proposal and voting modules.
pub fn setup_dao_v1(voting_type: VotingType) -> (App, ModuleAddrs, V1CodeIds) {
    let mut app = App::default();
    let sender = Addr::unchecked(SENDER_ADDR);

    let (core_addr, v1_code_ids) = init_v1(app.borrow_mut(), sender.clone(), voting_type.clone());
    let module_addrs = get_module_addrs(app.borrow_mut(), core_addr);

    match voting_type {
        VotingType::Cw4 => set_dummy_proposal(app.borrow_mut(), sender, module_addrs.clone()),
        VotingType::Cw20 => set_cw20_to_dao(app.borrow_mut(), sender, module_addrs.clone()),
    };

    (app, module_addrs, v1_code_ids)
}

pub fn execute_migration(
    app: &mut App,
    module_addrs: &ModuleAddrs,
    v1_code_ids: V1CodeIds,
    params: Option<ExecuteParams>,
) -> Result<AppResponse, anyhow::Error> {
    let sender = Addr::unchecked(SENDER_ADDR);
    let migrator_code_id = app.store_code(migrator_contract());
    let (new_code_ids, v2_code_ids) = get_v2_code_ids(app);
    let params = params.unwrap_or_else(|| ExecuteParams {
        sub_daos: Some(vec![]),
        migrate_cw20: Some(true),
    });

    app.execute_contract(
        sender.clone(),
        module_addrs.proposal.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Propose {
            title: "t2".to_string(),
            description: "d2".to_string(),
            msgs: vec![
                WasmMsg::Migrate {
                    contract_addr: module_addrs.core.to_string(),
                    new_code_id: new_code_ids.core,
                    msg: to_binary(&dao_core::msg::MigrateMsg::FromV1 { dao_uri: None, params: None }).unwrap(),
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: module_addrs.core.to_string(),
                    msg: to_binary(&dao_core::msg::ExecuteMsg::UpdateProposalModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: migrator_code_id,
                            msg: to_binary(&crate::msg::InstantiateMsg {
                                sub_daos: params.sub_daos.unwrap(),
                                migration_params: MigrationParams {
                                    migrate_stake_cw20_manager: params.migrate_cw20,
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

    let perposals: cw_proposal_single_v1::query::ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            module_addrs.proposal.clone(),
            &cw_proposal_single_v1::msg::QueryMsg::ReverseProposals {
                start_before: None,
                limit: Some(1),
            },
        )
        .unwrap();
    let proposal_id = perposals.proposals.first().unwrap().id;

    app.execute_contract(
        sender.clone(),
        module_addrs.proposal.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Vote {
            proposal_id,
            vote: voting_v1::Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender,
        module_addrs.proposal.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Execute { proposal_id },
        &[],
    )
}

pub fn execute_migration_from_core(
    app: &mut App,
    module_addrs: &ModuleAddrs,
    v1_code_ids: V1CodeIds,
    params: Option<ExecuteParams>,
) -> Result<AppResponse, anyhow::Error> {
    let sender = Addr::unchecked(SENDER_ADDR);
    let migrator_code_id = app.store_code(migrator_contract());
    let (new_code_ids, v2_code_ids) = get_v2_code_ids(app);
    let params = params.unwrap_or_else(|| ExecuteParams {
        sub_daos: Some(vec![]),
        migrate_cw20: Some(true),
    });

    app.execute_contract(
        sender.clone(),
        module_addrs.proposal.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Propose {
            title: "t2".to_string(),
            description: "d2".to_string(),
            msgs: vec![WasmMsg::Migrate {
                contract_addr: module_addrs.core.to_string(),
                new_code_id: new_code_ids.core,
                msg: to_binary(&dao_core::msg::MigrateMsg::FromV1 {
                    dao_uri: None,
                    params: Some(dao_core::migrate_msg::MigrateParams {
                        migrator_code_id,
                        params: dao_core::migrate_msg::MigrateV1ToV2 {
                            sub_daos: params.sub_daos.unwrap(),
                            migration_params: dao_core::migrate_msg::MigrationModuleParams {
                                migrate_stake_cw20_manager: params.migrate_cw20,
                                close_proposal_on_execution_failure: true,
                                pre_propose_info:
                                    dao_core::migrate_msg::PreProposeInfo::AnyoneMayPropose {},
                            },
                            v1_code_ids: v1_code_ids.to(),
                            v2_code_ids: v2_code_ids.to(),
                        },
                    }),
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let perposals: cw_proposal_single_v1::query::ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            module_addrs.proposal.clone(),
            &cw_proposal_single_v1::msg::QueryMsg::ReverseProposals {
                start_before: None,
                limit: Some(1),
            },
        )
        .unwrap();
    let proposal_id = perposals.proposals.first().unwrap().id;

    app.execute_contract(
        sender.clone(),
        module_addrs.proposal.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Vote {
            proposal_id,
            vote: voting_v1::Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender,
        module_addrs.proposal.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Execute { proposal_id },
        &[],
    )
}
