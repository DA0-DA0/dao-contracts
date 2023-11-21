use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env},
    to_json_binary, Addr, CosmosMsg, Empty, Storage, Uint128, WasmMsg,
};
use cw2::{set_contract_version, ContractVersion};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};
use dao_interface::{
    msg::{ExecuteMsg, InitialItem, InstantiateMsg, MigrateMsg, QueryMsg},
    query::{
        AdminNominationResponse, Cw20BalanceResponse, DaoURIResponse, DumpStateResponse,
        GetItemResponse, PauseInfoResponse, ProposalModuleCountResponse, SubDao,
    },
    state::{Admin, Config, ModuleInstantiateInfo, ProposalModule, ProposalModuleStatus},
    voting::{InfoResponse, VotingPowerAtHeightResponse},
};

use crate::{
    contract::{derive_proposal_module_prefix, migrate, CONTRACT_NAME, CONTRACT_VERSION},
    state::PROPOSAL_MODULES,
    ContractError,
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

fn cw721_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

fn sudo_proposal_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_sudo::contract::execute,
        dao_proposal_sudo::contract::instantiate,
        dao_proposal_sudo::contract::query,
    );
    Box::new(contract)
}

fn cw20_balances_voting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_voting_cw20_balance::contract::execute,
        dao_voting_cw20_balance::contract::instantiate,
        dao_voting_cw20_balance::contract::query,
    )
    .with_reply(dao_voting_cw20_balance::contract::reply);
    Box::new(contract)
}

fn cw_core_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply)
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}

fn v1_cw_core_contract() -> Box<dyn Contract<Empty>> {
    use cw_core_v1::contract;
    let contract = ContractWrapper::new(contract::execute, contract::instantiate, contract::query)
        .with_reply(contract::reply)
        .with_migrate(contract::migrate);
    Box::new(contract)
}

fn instantiate_gov(app: &mut App, code_id: u64, msg: InstantiateMsg) -> Addr {
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

fn test_instantiate_with_n_gov_modules(n: usize) {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_contract());
    let gov_id = app.store_code(cw_core_contract());

    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "DAO".to_string(),
        symbol: "DAO".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
    };
    let instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_id,
            msg: to_json_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: (0..n)
            .map(|n| ModuleInstantiateInfo {
                code_id: cw20_id,
                msg: to_json_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: format!("governance module {n}"),
            })
            .collect(),
        initial_items: None,
    };
    let gov_addr = instantiate_gov(&mut app, gov_id, instantiate);

    let state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(gov_addr, &QueryMsg::DumpState {})
        .unwrap();

    assert_eq!(
        state.config,
        Config {
            dao_uri: None,
            name: "DAO DAO".to_string(),
            description: "A DAO that builds DAOs.".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
        }
    );

    assert_eq!(state.proposal_modules.len(), n);

    assert_eq!(state.active_proposal_module_count, n as u32);
    assert_eq!(state.total_proposal_module_count, n as u32);
}

#[test]
#[should_panic(expected = "Execution would result in no proposal modules being active.")]
fn test_instantiate_with_zero_gov_modules() {
    test_instantiate_with_n_gov_modules(0)
}

#[test]
fn test_valid_instantiate() {
    let module_counts = [1, 2, 200];
    for count in module_counts {
        test_instantiate_with_n_gov_modules(count)
    }
}

#[test]
#[should_panic(expected = "Error parsing into type cw20_base::msg::InstantiateMsg: Invalid type")]
fn test_instantiate_with_submessage_failure() {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_contract());
    let gov_id = app.store_code(cw_core_contract());

    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "DAO".to_string(),
        symbol: "DAO".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
    };

    let mut governance_modules = (0..3)
        .map(|n| ModuleInstantiateInfo {
            code_id: cw20_id,
            msg: to_json_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: format!("governance module {n}"),
        })
        .collect::<Vec<_>>();
    governance_modules.push(ModuleInstantiateInfo {
        code_id: cw20_id,
        msg: to_json_binary("bad").unwrap(),
        admin: Some(Admin::CoreModule {}),
        funds: vec![],
        label: "I have a bad instantiate message".to_string(),
    });
    governance_modules.push(ModuleInstantiateInfo {
        code_id: cw20_id,
        msg: to_json_binary(&cw20_instantiate).unwrap(),
        admin: Some(Admin::CoreModule {}),
        funds: vec![],
        label: "Everybody knowing
that goodness is good
makes wickedness."
            .to_string(),
    });

    let instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_id,
            msg: to_json_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: governance_modules,
        initial_items: None,
    };
    instantiate_gov(&mut app, gov_id, instantiate);
}

#[test]
fn test_update_config() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    let expected_config = Config {
        name: "Root DAO".to_string(),
        description: "We love trees and sudo.".to_string(),
        image_url: Some("https://moonphase.is/image.svg".to_string()),
        automatically_add_cw20s: false,
        automatically_add_cw721s: true,
        dao_uri: Some("https://daostar.one/EIP".to_string()),
    };

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        modules[0].clone().address,
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&ExecuteMsg::UpdateConfig {
                    config: expected_config.clone(),
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let config: Config = app
        .wrap()
        .query_wasm_smart(gov_addr.clone(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(expected_config, config);

    let dao_uri: DaoURIResponse = app
        .wrap()
        .query_wasm_smart(gov_addr, &QueryMsg::DaoURI {})
        .unwrap();
    assert_eq!(dao_uri.dao_uri, expected_config.dao_uri);
}

fn test_swap_governance(swaps: Vec<(u32, u32)>) {
    let mut app = App::default();
    let propmod_id = app.store_code(sudo_proposal_contract());
    let core_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: propmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: propmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            core_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    let module_count = query_proposal_module_count(&app, &gov_addr);
    assert_eq!(
        module_count,
        ProposalModuleCountResponse {
            active_proposal_module_count: 1,
            total_proposal_module_count: 1,
        }
    );

    let (to_add, to_remove) = swaps
        .iter()
        .cloned()
        .reduce(|(to_add, to_remove), (add, remove)| (to_add + add, to_remove + remove))
        .unwrap_or((0, 0));

    for (add, remove) in swaps {
        let start_modules: Vec<ProposalModule> = app
            .wrap()
            .query_wasm_smart(
                gov_addr.clone(),
                &QueryMsg::ProposalModules {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        let start_modules_active: Vec<ProposalModule> = get_active_modules(&app, gov_addr.clone());

        let to_add: Vec<_> = (0..add)
            .map(|n| ModuleInstantiateInfo {
                code_id: propmod_id,
                msg: to_json_binary(&govmod_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: format!("governance module {n}"),
            })
            .collect();

        let to_disable: Vec<_> = start_modules_active
            .iter()
            .rev()
            .take(remove as usize)
            .map(|a| a.address.to_string())
            .collect();

        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            start_modules_active[0].address.clone(),
            &dao_proposal_sudo::msg::ExecuteMsg::Execute {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: gov_addr.to_string(),
                    funds: vec![],
                    msg: to_json_binary(&ExecuteMsg::UpdateProposalModules { to_add, to_disable })
                        .unwrap(),
                }
                .into()],
            },
            &[],
        )
        .unwrap();

        let finish_modules_active = get_active_modules(&app, gov_addr.clone());

        assert_eq!(
            finish_modules_active.len() as u32,
            start_modules_active.len() as u32 + add - remove
        );
        for module in start_modules
            .clone()
            .into_iter()
            .rev()
            .take(remove as usize)
        {
            assert!(!finish_modules_active.contains(&module))
        }

        let state: DumpStateResponse = app
            .wrap()
            .query_wasm_smart(gov_addr.clone(), &QueryMsg::DumpState {})
            .unwrap();

        assert_eq!(
            state.active_proposal_module_count,
            finish_modules_active.len() as u32
        );

        assert_eq!(
            state.total_proposal_module_count,
            start_modules.len() as u32 + add
        )
    }

    let module_count = query_proposal_module_count(&app, &gov_addr);
    assert_eq!(
        module_count,
        ProposalModuleCountResponse {
            active_proposal_module_count: 1 + to_add - to_remove,
            total_proposal_module_count: 1 + to_add,
        }
    );
}

#[test]
fn test_update_governance() {
    test_swap_governance(vec![(1, 1), (5, 0), (0, 5), (0, 0)]);
    test_swap_governance(vec![(1, 1), (1, 1), (1, 1), (1, 1)])
}

#[test]
fn test_add_then_remove_governance() {
    test_swap_governance(vec![(1, 0), (0, 1)])
}

#[test]
#[should_panic(expected = "Execution would result in no proposal modules being active.")]
fn test_swap_governance_bad() {
    test_swap_governance(vec![(1, 1), (0, 1)])
}

#[test]
fn test_removed_modules_can_not_execute() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    let start_module = modules.into_iter().next().unwrap();

    let to_add = vec![ModuleInstantiateInfo {
        code_id: govmod_id,
        msg: to_json_binary(&govmod_instantiate).unwrap(),
        admin: Some(Admin::CoreModule {}),
        funds: vec![],
        label: "new governance module".to_string(),
    }];

    let to_disable = vec![start_module.address.to_string()];

    // Swap ourselves out.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        start_module.address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&ExecuteMsg::UpdateProposalModules { to_add, to_disable })
                    .unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let finish_modules_active: Vec<ProposalModule> = get_active_modules(&app, gov_addr.clone());

    let new_proposal_module = finish_modules_active.into_iter().next().unwrap();

    // Try to add a new module and remove the one we added
    // earlier. This should fail as we have been removed.
    let to_add = vec![ModuleInstantiateInfo {
        code_id: govmod_id,
        msg: to_json_binary(&govmod_instantiate).unwrap(),
        admin: Some(Admin::CoreModule {}),
        funds: vec![],
        label: "new governance module".to_string(),
    }];
    let to_disable = vec![new_proposal_module.address.to_string()];

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            start_module.address,
            &dao_proposal_sudo::msg::ExecuteMsg::Execute {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: gov_addr.to_string(),
                    funds: vec![],
                    msg: to_json_binary(&ExecuteMsg::UpdateProposalModules {
                        to_add: to_add.clone(),
                        to_disable: to_disable.clone(),
                    })
                    .unwrap(),
                }
                .into()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(
        err,
        ContractError::ModuleDisabledCannotExecute {
            address: _gov_address
        }
    ));

    // Check that the enabled query works.
    let enabled_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            &gov_addr,
            &QueryMsg::ActiveProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(enabled_modules, vec![new_proposal_module.clone()]);

    // The new proposal module should be able to perform actions.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        new_proposal_module.address,
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&ExecuteMsg::UpdateProposalModules { to_add, to_disable })
                    .unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();
}

#[test]
fn test_module_already_disabled() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    let start_module = modules.into_iter().next().unwrap();

    let to_disable = vec![
        start_module.address.to_string(),
        start_module.address.to_string(),
    ];

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            start_module.address.clone(),
            &dao_proposal_sudo::msg::ExecuteMsg::Execute {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: gov_addr.to_string(),
                    funds: vec![],
                    msg: to_json_binary(&ExecuteMsg::UpdateProposalModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: govmod_id,
                            msg: to_json_binary(&govmod_instantiate).unwrap(),
                            admin: Some(Admin::CoreModule {}),
                            funds: vec![],
                            label: "governance module".to_string(),
                        }],
                        to_disable,
                    })
                    .unwrap(),
                }
                .into()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        err,
        ContractError::ModuleAlreadyDisabled {
            address: start_module.address
        }
    )
}

#[test]
fn test_swap_voting_module() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let voting_addr: Addr = app
        .wrap()
        .query_wasm_smart(gov_addr.clone(), &QueryMsg::VotingModule {})
        .unwrap();

    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        modules[0].address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&ExecuteMsg::UpdateVotingModule {
                    module: ModuleInstantiateInfo {
                        code_id: govmod_id,
                        msg: to_json_binary(&govmod_instantiate).unwrap(),
                        admin: Some(Admin::CoreModule {}),
                        funds: vec![],
                        label: "voting module".to_string(),
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let new_voting_addr: Addr = app
        .wrap()
        .query_wasm_smart(gov_addr, &QueryMsg::VotingModule {})
        .unwrap();

    assert_ne!(new_voting_addr, voting_addr);
}

fn test_unauthorized(app: &mut App, gov_addr: Addr, msg: ExecuteMsg) {
    let err: ContractError = app
        .execute_contract(Addr::unchecked(CREATOR_ADDR), gov_addr, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_permissions() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    test_unauthorized(
        &mut app,
        gov_addr.clone(),
        ExecuteMsg::UpdateVotingModule {
            module: ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_json_binary(&govmod_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "voting module".to_string(),
            },
        },
    );

    test_unauthorized(
        &mut app,
        gov_addr.clone(),
        ExecuteMsg::UpdateProposalModules {
            to_add: vec![],
            to_disable: vec![],
        },
    );

    test_unauthorized(
        &mut app,
        gov_addr,
        ExecuteMsg::UpdateConfig {
            config: Config {
                dao_uri: None,
                name: "Evil config.".to_string(),
                description: "👿".to_string(),
                image_url: None,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
            },
        },
    );
}

fn do_standard_instantiate(auto_add: bool, admin: Option<String>) -> (Addr, App) {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = dao_voting_cw20_balance::msg::InstantiateMsg {
        token_info: dao_voting_cw20_balance::msg::TokenInfo::New {
            code_id: cw20_id,
            label: "DAO DAO voting".to_string(),
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: CREATOR_ADDR.to_string(),
                amount: Uint128::from(2u64),
            }],
            marketing: None,
        },
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: auto_add,
        automatically_add_cw721s: auto_add,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_json_binary(&voting_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    (gov_addr, app)
}

#[test]
fn test_admin_permissions() {
    let (core_addr, mut app) = do_standard_instantiate(true, None);

    let start_height = app.block_info().height;
    let proposal_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(proposal_modules.len(), 1);
    let proposal_module = proposal_modules.into_iter().next().unwrap();

    // Random address can't call ExecuteAdminMsgs
    let res = app.execute_contract(
        Addr::unchecked("random"),
        core_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    res.unwrap_err();

    // Proposal mdoule can't call ExecuteAdminMsgs
    let res = app.execute_contract(
        proposal_module.address.clone(),
        core_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    res.unwrap_err();

    // Update Admin can't be called by non-admins
    let res = app.execute_contract(
        Addr::unchecked("rando"),
        core_addr.clone(),
        &ExecuteMsg::NominateAdmin {
            admin: Some("rando".to_string()),
        },
        &[],
    );
    res.unwrap_err();

    // Nominate admin can be called by core contract as no admin was
    // specified so the admin defaulted to the core contract.
    let res = app.execute_contract(
        proposal_module.address.clone(),
        core_addr.clone(),
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::NominateAdmin {
                    admin: Some("meow".to_string()),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    res.unwrap();

    // Instantiate new DAO with an admin
    let (core_with_admin_addr, mut app) =
        do_standard_instantiate(true, Some(Addr::unchecked("admin").to_string()));

    // Non admins still can't call ExecuteAdminMsgs
    let res = app.execute_contract(
        proposal_module.address,
        core_with_admin_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_with_admin_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    res.unwrap_err();

    // Admin can call ExecuteAdminMsgs, here an admin pasues the DAO
    let res = app.execute_contract(
        Addr::unchecked("admin"),
        core_with_admin_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_with_admin_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    res.unwrap();

    let paused: PauseInfoResponse = app
        .wrap()
        .query_wasm_smart(core_with_admin_addr.clone(), &QueryMsg::PauseInfo {})
        .unwrap();
    assert_eq!(
        paused,
        PauseInfoResponse::Paused {
            expiration: Expiration::AtHeight(start_height + 10)
        }
    );

    // DAO unpauses after 10 blocks
    app.update_block(|block| block.height += 11);

    // Admin can nominate a new admin.
    let res = app.execute_contract(
        Addr::unchecked("admin"),
        core_with_admin_addr.clone(),
        &ExecuteMsg::NominateAdmin {
            admin: Some("meow".to_string()),
        },
        &[],
    );
    res.unwrap();

    let nomination: AdminNominationResponse = app
        .wrap()
        .query_wasm_smart(core_with_admin_addr.clone(), &QueryMsg::AdminNomination {})
        .unwrap();
    assert_eq!(
        nomination,
        AdminNominationResponse {
            nomination: Some(Addr::unchecked("meow"))
        }
    );

    // Check that admin has not yet been updated
    let res: Addr = app
        .wrap()
        .query_wasm_smart(core_with_admin_addr.clone(), &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(res, Addr::unchecked("admin"));

    // Only the nominated address may accept the nomination.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("random"),
            core_with_admin_addr.clone(),
            &ExecuteMsg::AcceptAdminNomination {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Accept the nomination.
    app.execute_contract(
        Addr::unchecked("meow"),
        core_with_admin_addr.clone(),
        &ExecuteMsg::AcceptAdminNomination {},
        &[],
    )
    .unwrap();

    // Check that admin has been updated
    let res: Addr = app
        .wrap()
        .query_wasm_smart(core_with_admin_addr.clone(), &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(res, Addr::unchecked("meow"));

    // Check that the pending admin has been cleared.
    let nomination: AdminNominationResponse = app
        .wrap()
        .query_wasm_smart(core_with_admin_addr, &QueryMsg::AdminNomination {})
        .unwrap();
    assert_eq!(nomination, AdminNominationResponse { nomination: None });
}

#[test]
fn test_admin_nomination() {
    let (core_addr, mut app) = do_standard_instantiate(true, Some("admin".to_string()));

    // Check that there is no pending nominations.
    let nomination: AdminNominationResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::AdminNomination {})
        .unwrap();
    assert_eq!(nomination, AdminNominationResponse { nomination: None });

    // Nominate a new admin.
    app.execute_contract(
        Addr::unchecked("admin"),
        core_addr.clone(),
        &ExecuteMsg::NominateAdmin {
            admin: Some("ekez".to_string()),
        },
        &[],
    )
    .unwrap();

    // Check that the nomination is in place.
    let nomination: AdminNominationResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::AdminNomination {})
        .unwrap();
    assert_eq!(
        nomination,
        AdminNominationResponse {
            nomination: Some(Addr::unchecked("ekez"))
        }
    );

    // Non-admin can not withdraw.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            core_addr.clone(),
            &ExecuteMsg::WithdrawAdminNomination {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Admin can withdraw.
    app.execute_contract(
        Addr::unchecked("admin"),
        core_addr.clone(),
        &ExecuteMsg::WithdrawAdminNomination {},
        &[],
    )
    .unwrap();

    // Check that the nomination is withdrawn.
    let nomination: AdminNominationResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::AdminNomination {})
        .unwrap();
    assert_eq!(nomination, AdminNominationResponse { nomination: None });

    // Can not withdraw if no nomination is pending.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("admin"),
            core_addr.clone(),
            &ExecuteMsg::WithdrawAdminNomination {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoAdminNomination {});

    // Can not claim nomination b/c it has been withdrawn.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            core_addr.clone(),
            &ExecuteMsg::AcceptAdminNomination {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoAdminNomination {});

    // Nominate a new admin.
    app.execute_contract(
        Addr::unchecked("admin"),
        core_addr.clone(),
        &ExecuteMsg::NominateAdmin {
            admin: Some("meow".to_string()),
        },
        &[],
    )
    .unwrap();

    // A new nomination can not be created if there is already a
    // pending nomination.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("admin"),
            core_addr.clone(),
            &ExecuteMsg::NominateAdmin {
                admin: Some("arthur".to_string()),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::PendingNomination {});

    // Only nominated admin may accept.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            core_addr.clone(),
            &ExecuteMsg::AcceptAdminNomination {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    app.execute_contract(
        Addr::unchecked("meow"),
        core_addr.clone(),
        &ExecuteMsg::AcceptAdminNomination {},
        &[],
    )
    .unwrap();

    // Check that meow is the new admin.
    let admin: Addr = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(admin, Addr::unchecked("meow".to_string()));

    let start_height = app.block_info().height;
    // Check that the new admin can do admin things and the old can not.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("admin"),
            core_addr.clone(),
            &ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: core_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Pause {
                        duration: Duration::Height(10),
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    let res = app.execute_contract(
        Addr::unchecked("meow"),
        core_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    res.unwrap();

    let paused: PauseInfoResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::PauseInfo {})
        .unwrap();
    assert_eq!(
        paused,
        PauseInfoResponse::Paused {
            expiration: Expiration::AtHeight(start_height + 10)
        }
    );

    // DAO unpauses after 10 blocks
    app.update_block(|block| block.height += 11);

    // Remove the admin.
    app.execute_contract(
        Addr::unchecked("meow"),
        core_addr.clone(),
        &ExecuteMsg::NominateAdmin { admin: None },
        &[],
    )
    .unwrap();

    // Check that this has not caused an admin to be nominated.
    let nomination: AdminNominationResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::AdminNomination {})
        .unwrap();
    assert_eq!(nomination, AdminNominationResponse { nomination: None });

    // Check that admin has been updated. As there was no admin
    // nominated the admin should revert back to the contract address.
    let res: Addr = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(res, core_addr);
}

#[test]
fn test_passthrough_voting_queries() {
    let (gov_addr, app) = do_standard_instantiate(true, None);

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::VotingPowerAtHeight {
                address: CREATOR_ADDR.to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(2u64),
            height: app.block_info().height,
        }
    );
}

fn set_item(app: &mut App, gov_addr: Addr, key: String, value: String) {
    app.execute_contract(
        gov_addr.clone(),
        gov_addr,
        &ExecuteMsg::SetItem { key, value },
        &[],
    )
    .unwrap();
}

fn remove_item(app: &mut App, gov_addr: Addr, key: String) {
    app.execute_contract(
        gov_addr.clone(),
        gov_addr,
        &ExecuteMsg::RemoveItem { key },
        &[],
    )
    .unwrap();
}

fn get_item(app: &mut App, gov_addr: Addr, key: String) -> GetItemResponse {
    app.wrap()
        .query_wasm_smart(gov_addr, &QueryMsg::GetItem { key })
        .unwrap()
}

fn list_items(
    app: &mut App,
    gov_addr: Addr,
    start_at: Option<String>,
    limit: Option<u32>,
) -> Vec<(String, String)> {
    app.wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::ListItems {
                start_after: start_at,
                limit,
            },
        )
        .unwrap()
}

#[test]
fn test_item_permissions() {
    let (gov_addr, mut app) = do_standard_instantiate(true, None);

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            gov_addr.clone(),
            &ExecuteMsg::SetItem {
                key: "k".to_string(),
                value: "v".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            gov_addr,
            &ExecuteMsg::RemoveItem {
                key: "k".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_add_remove_get() {
    let (gov_addr, mut app) = do_standard_instantiate(true, None);

    let a = get_item(&mut app, gov_addr.clone(), "aaaaa".to_string());
    assert_eq!(a, GetItemResponse { item: None });

    set_item(
        &mut app,
        gov_addr.clone(),
        "aaaaakey".to_string(),
        "aaaaaaddr".to_string(),
    );
    let a = get_item(&mut app, gov_addr.clone(), "aaaaakey".to_string());
    assert_eq!(
        a,
        GetItemResponse {
            item: Some("aaaaaaddr".to_string())
        }
    );

    remove_item(&mut app, gov_addr.clone(), "aaaaakey".to_string());
    let a = get_item(&mut app, gov_addr, "aaaaakey".to_string());
    assert_eq!(a, GetItemResponse { item: None });
}

#[test]
#[should_panic(expected = "Key is missing from storage")]
fn test_remove_missing_key() {
    let (gov_addr, mut app) = do_standard_instantiate(true, None);
    remove_item(&mut app, gov_addr, "b".to_string())
}

#[test]
fn test_list_items() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = dao_voting_cw20_balance::msg::InstantiateMsg {
        token_info: dao_voting_cw20_balance::msg::TokenInfo::New {
            code_id: cw20_id,
            label: "DAO DAO voting".to_string(),
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: CREATOR_ADDR.to_string(),
                amount: Uint128::from(2u64),
            }],
            marketing: None,
        },
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_json_binary(&voting_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    set_item(
        &mut app,
        gov_addr.clone(),
        "fookey".to_string(),
        "fooaddr".to_string(),
    );
    set_item(
        &mut app,
        gov_addr.clone(),
        "barkey".to_string(),
        "baraddr".to_string(),
    );
    set_item(
        &mut app,
        gov_addr.clone(),
        "loremkey".to_string(),
        "loremaddr".to_string(),
    );
    set_item(
        &mut app,
        gov_addr.clone(),
        "ipsumkey".to_string(),
        "ipsumaddr".to_string(),
    );

    // Foo returned as we are only getting one item and items are in
    // decending order.
    let first_item = list_items(&mut app, gov_addr.clone(), None, Some(1));
    assert_eq!(first_item.len(), 1);
    assert_eq!(
        first_item[0],
        ("loremkey".to_string(), "loremaddr".to_string())
    );

    let no_items = list_items(&mut app, gov_addr.clone(), None, Some(0));
    assert_eq!(no_items.len(), 0);

    // Items are retreived in decending order so asking for foo with
    // no limit ought to give us the barkey k/v. this will be the last item
    // note: the paginate map bound is exclusive, so fookey will be starting point
    let last_item = list_items(&mut app, gov_addr.clone(), Some("foo".to_string()), None);
    assert_eq!(last_item.len(), 1);
    assert_eq!(last_item[0], ("barkey".to_string(), "baraddr".to_string()));

    // Items are retreived in decending order so asking for ipsum with
    // 4 limit ought to give us the fookey and barkey k/vs.
    let after_foo_list = list_items(&mut app, gov_addr, Some("ipsum".to_string()), Some(4));
    assert_eq!(after_foo_list.len(), 2);
    assert_eq!(
        after_foo_list,
        vec![
            ("fookey".to_string(), "fooaddr".to_string()),
            ("barkey".to_string(), "baraddr".to_string())
        ]
    );
}

#[test]
fn test_instantiate_with_items() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = dao_voting_cw20_balance::msg::InstantiateMsg {
        token_info: dao_voting_cw20_balance::msg::TokenInfo::New {
            code_id: cw20_id,
            label: "DAO DAO voting".to_string(),
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: CREATOR_ADDR.to_string(),
                amount: Uint128::from(2u64),
            }],
            marketing: None,
        },
    };

    let mut initial_items = vec![
        InitialItem {
            key: "item0".to_string(),
            value: "item0_value".to_string(),
        },
        InitialItem {
            key: "item1".to_string(),
            value: "item1_value".to_string(),
        },
        InitialItem {
            key: "item0".to_string(),
            value: "item0_value_override".to_string(),
        },
    ];

    let mut gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_json_binary(&voting_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: Some(initial_items.clone()),
    };

    // Ensure duplicates are dissallowed.
    let err: ContractError = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::DuplicateInitialItem {
            item: "item0".to_string()
        }
    );

    initial_items.pop();
    gov_instantiate.initial_items = Some(initial_items);
    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    // Ensure initial items were added.
    let items = list_items(&mut app, gov_addr.clone(), None, None);
    assert_eq!(items.len(), 2);

    // Descending order, so item1 is first.
    assert_eq!(items[1].0, "item0".to_string());
    let get_item0 = get_item(&mut app, gov_addr.clone(), "item0".to_string());
    assert_eq!(
        get_item0,
        GetItemResponse {
            item: Some("item0_value".to_string()),
        }
    );

    assert_eq!(items[0].0, "item1".to_string());
    let item1_value = get_item(&mut app, gov_addr, "item1".to_string()).item;
    assert_eq!(item1_value, Some("item1_value".to_string()))
}

#[test]
fn test_cw20_receive_auto_add() {
    let (gov_addr, mut app) = do_standard_instantiate(true, None);

    let cw20_id = app.store_code(cw20_contract());
    let another_cw20 = app
        .instantiate_contract(
            cw20_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: None,
                marketing: None,
            },
            &[],
            "another-token",
            None,
        )
        .unwrap();

    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(gov_addr.clone(), &QueryMsg::VotingModule {})
        .unwrap();
    let gov_token: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &dao_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Check that the balances query works with no tokens.
    let cw20_balances: Vec<Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_balances, vec![]);

    // Send a gov token to the governance contract.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        gov_token.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: gov_addr.to_string(),
            amount: Uint128::new(1),
            msg: to_json_binary(&"").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![gov_token.clone()]);

    let cw20_balances: Vec<Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        cw20_balances,
        vec![Cw20BalanceResponse {
            addr: gov_token.clone(),
            balance: Uint128::new(1),
        }]
    );

    // Test removing and adding some new ones. Invalid should fail.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(gov_addr.clone()),
            gov_addr.clone(),
            &ExecuteMsg::UpdateCw20List {
                to_add: vec!["new".to_string()],
                to_remove: vec![gov_token.to_string()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::Std(_)));

    // Test that non-DAO can not update the list.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            gov_addr.clone(),
            &ExecuteMsg::UpdateCw20List {
                to_add: vec![],
                to_remove: vec![gov_token.to_string()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::Unauthorized {}));

    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw20List {
            to_add: vec![another_cw20.to_string()],
            to_remove: vec![gov_token.to_string()],
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::Cw20TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![another_cw20]);
}

#[test]
fn test_cw20_receive_no_auto_add() {
    let (gov_addr, mut app) = do_standard_instantiate(false, None);

    let cw20_id = app.store_code(cw20_contract());
    let another_cw20 = app
        .instantiate_contract(
            cw20_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: None,
                marketing: None,
            },
            &[],
            "another-token",
            None,
        )
        .unwrap();

    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(gov_addr.clone(), &QueryMsg::VotingModule {})
        .unwrap();
    let gov_token: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &dao_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Send a gov token to the governance contract. Should not be
    // added becasue auto add is turned off.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        gov_token.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: gov_addr.to_string(),
            amount: Uint128::new(1),
            msg: to_json_binary(&"").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, Vec::<Addr>::new());

    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw20List {
            to_add: vec![another_cw20.to_string(), gov_token.to_string()],
            to_remove: vec!["ok to remove non existent".to_string()],
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::Cw20TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![another_cw20, gov_token]);
}

#[test]
fn test_cw721_receive() {
    let (gov_addr, mut app) = do_standard_instantiate(true, None);

    let cw721_id = app.store_code(cw721_contract());

    let cw721_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw721_base::msg::InstantiateMsg {
                name: "ekez".to_string(),
                symbol: "ekez".to_string(),
                minter: CREATOR_ADDR.to_string(),
            },
            &[],
            "cw721",
            None,
        )
        .unwrap();

    let another_cw721 = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw721_base::msg::InstantiateMsg {
                name: "ekez".to_string(),
                symbol: "ekez".to_string(),
                minter: CREATOR_ADDR.to_string(),
            },
            &[],
            "cw721",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::<Option<Empty>, Empty>::Mint {
            token_id: "ekez".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::<Option<Empty>, Empty>::SendNft {
            contract: gov_addr.to_string(),
            token_id: "ekez".to_string(),
            msg: to_json_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw721_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw721TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw721_list, vec![cw721_addr.clone()]);

    // Try to add an invalid cw721.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(gov_addr.clone()),
            gov_addr.clone(),
            &ExecuteMsg::UpdateCw721List {
                to_add: vec!["new".to_string(), cw721_addr.to_string()],
                to_remove: vec![cw721_addr.to_string()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::Std(_)));

    // Test that non-DAO can not update the list.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            gov_addr.clone(),
            &ExecuteMsg::UpdateCw721List {
                to_add: vec![],
                to_remove: vec![cw721_addr.to_string()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Add a real cw721.
    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw721List {
            to_add: vec![another_cw721.to_string(), cw721_addr.to_string()],
            to_remove: vec![cw721_addr.to_string()],
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::Cw721TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![another_cw721]);
}

#[test]
fn test_cw721_receive_no_auto_add() {
    let (gov_addr, mut app) = do_standard_instantiate(false, None);

    let cw721_id = app.store_code(cw721_contract());

    let cw721_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw721_base::msg::InstantiateMsg {
                name: "ekez".to_string(),
                symbol: "ekez".to_string(),
                minter: CREATOR_ADDR.to_string(),
            },
            &[],
            "cw721",
            None,
        )
        .unwrap();

    let another_cw721 = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw721_base::msg::InstantiateMsg {
                name: "ekez".to_string(),
                symbol: "ekez".to_string(),
                minter: CREATOR_ADDR.to_string(),
            },
            &[],
            "cw721",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::<Option<Empty>, Empty>::Mint {
            token_id: "ekez".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::<Option<Empty>, Empty>::SendNft {
            contract: gov_addr.to_string(),
            token_id: "ekez".to_string(),
            msg: to_json_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw721_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw721TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw721_list, Vec::<Addr>::new());

    // Duplicates OK. Just adds one.
    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw721List {
            to_add: vec![
                another_cw721.to_string(),
                cw721_addr.to_string(),
                cw721_addr.to_string(),
            ],
            to_remove: vec![],
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::Cw721TokenList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![another_cw721, cw721_addr]);
}

#[test]
fn test_pause() {
    let (core_addr, mut app) = do_standard_instantiate(false, None);

    let start_height = app.block_info().height;

    let proposal_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(proposal_modules.len(), 1);
    let proposal_module = proposal_modules.into_iter().next().unwrap();

    let paused: PauseInfoResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::PauseInfo {})
        .unwrap();
    assert_eq!(paused, PauseInfoResponse::Unpaused {});
    let all_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::DumpState {})
        .unwrap();
    assert_eq!(all_state.pause_info, PauseInfoResponse::Unpaused {});

    // DAO is not paused. Check that we can execute things.
    //
    // Tests intentionally use the core address to send these
    // messsages to simulate a worst case scenerio where the core
    // contract has a vulnerability.
    app.execute_contract(
        core_addr.clone(),
        core_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            config: Config {
                dao_uri: None,
                name: "The Empire Strikes Back".to_string(),
                description: "haha lol we have pwned your DAO".to_string(),
                image_url: None,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
            },
        },
        &[],
    )
    .unwrap();

    // Oh no the DAO is under attack! Quick! Pause the DAO while we
    // figure out what to do!
    let err: ContractError = app
        .execute_contract(
            proposal_module.address.clone(),
            core_addr.clone(),
            &ExecuteMsg::Pause {
                duration: Duration::Height(10),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    // Only the DAO may call this on itself. Proposal modules must use
    // the execute hook.
    assert_eq!(err, ContractError::Unauthorized {});

    app.execute_contract(
        proposal_module.address.clone(),
        core_addr.clone(),
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let paused: PauseInfoResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::PauseInfo {})
        .unwrap();
    assert_eq!(
        paused,
        PauseInfoResponse::Paused {
            expiration: Expiration::AtHeight(start_height + 10)
        }
    );
    let all_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::DumpState {})
        .unwrap();
    assert_eq!(
        all_state.pause_info,
        PauseInfoResponse::Paused {
            expiration: Expiration::AtHeight(start_height + 10)
        }
    );

    let err: ContractError = app
        .execute_contract(
            core_addr.clone(),
            core_addr.clone(),
            &ExecuteMsg::UpdateConfig {
                config: Config {
                    dao_uri: None,
                    name: "The Empire Strikes Back Again".to_string(),
                    description: "haha lol we have pwned your DAO again".to_string(),
                    image_url: None,
                    automatically_add_cw20s: true,
                    automatically_add_cw721s: true,
                },
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::Paused { .. }));

    let err: ContractError = app
        .execute_contract(
            proposal_module.address.clone(),
            core_addr.clone(),
            &ExecuteMsg::ExecuteProposalHook {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: core_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Pause {
                        duration: Duration::Height(10),
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::Paused { .. }));

    app.update_block(|block| block.height += 9);

    // Still not unpaused.
    let err: ContractError = app
        .execute_contract(
            proposal_module.address.clone(),
            core_addr.clone(),
            &ExecuteMsg::ExecuteProposalHook {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: core_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Pause {
                        duration: Duration::Height(10),
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::Paused { .. }));

    app.update_block(|block| block.height += 1);

    let paused: PauseInfoResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::PauseInfo {})
        .unwrap();
    assert_eq!(paused, PauseInfoResponse::Unpaused {});
    let all_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::DumpState {})
        .unwrap();
    assert_eq!(all_state.pause_info, PauseInfoResponse::Unpaused {});

    // Now its unpaused so we should be able to pause again.
    app.execute_contract(
        proposal_module.address,
        core_addr.clone(),
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let paused: PauseInfoResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::PauseInfo {})
        .unwrap();
    assert_eq!(
        paused,
        PauseInfoResponse::Paused {
            expiration: Expiration::AtHeight(start_height + 20)
        }
    );
    let all_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &QueryMsg::DumpState {})
        .unwrap();
    assert_eq!(
        all_state.pause_info,
        PauseInfoResponse::Paused {
            expiration: Expiration::AtHeight(start_height + 20)
        }
    );
}

#[test]
fn test_dump_state_proposal_modules() {
    let (core_addr, app) = do_standard_instantiate(false, None);
    let proposal_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(proposal_modules.len(), 1);
    let proposal_module = proposal_modules.into_iter().next().unwrap();

    let all_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &QueryMsg::DumpState {})
        .unwrap();
    assert_eq!(all_state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(all_state.proposal_modules.len(), 1);
    assert_eq!(all_state.proposal_modules[0], proposal_module);
}

// Note that this isn't actually testing that we are migrating from the previous version since
// with multitest contract instantiation we can't manipulate storage to the previous version of state before invoking migrate. So if anything,
// this just tests the idempotency of migrate.
#[test]
fn test_migrate_from_compatible() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = dao_voting_cw20_balance::msg::InstantiateMsg {
        token_info: dao_voting_cw20_balance::msg::TokenInfo::New {
            code_id: cw20_id,
            label: "DAO DAO voting".to_string(),
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: CREATOR_ADDR.to_string(),
                amount: Uint128::from(2u64),
            }],
            marketing: None,
        },
    };

    // Instantiate the core module with an admin to do migrations.
    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_json_binary(&voting_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "governance module".to_string(),
        }],
        initial_items: None,
    };

    let core_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            Some(CREATOR_ADDR.to_string()),
        )
        .unwrap();

    let state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr.clone(), &QueryMsg::DumpState {})
        .unwrap();

    app.execute(
        Addr::unchecked(CREATOR_ADDR),
        CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: core_addr.to_string(),
            new_code_id: gov_id,
            msg: to_json_binary(&MigrateMsg::FromCompatible {}).unwrap(),
        }),
    )
    .unwrap();

    let new_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &QueryMsg::DumpState {})
        .unwrap();

    assert_eq!(new_state, state);
}

#[test]
fn test_migrate_from_beta() {
    use cw_core_v1 as v1;

    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let core_id = app.store_code(cw_core_contract());
    let v1_core_id = app.store_code(v1_cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let proposal_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = dao_voting_cw20_balance::msg::InstantiateMsg {
        token_info: dao_voting_cw20_balance::msg::TokenInfo::New {
            code_id: cw20_id,
            label: "DAO DAO voting".to_string(),
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: CREATOR_ADDR.to_string(),
                amount: Uint128::from(2u64),
            }],
            marketing: None,
        },
    };

    // Instantiate the core module with an admin to do migrations.
    let v1_core_instantiate = v1::msg::InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: v1::msg::ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_json_binary(&voting_instantiate).unwrap(),
            admin: v1::msg::Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![
            v1::msg::ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_json_binary(&proposal_instantiate).unwrap(),
                admin: v1::msg::Admin::CoreContract {},
                label: "governance module 1".to_string(),
            },
            v1::msg::ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_json_binary(&proposal_instantiate).unwrap(),
                admin: v1::msg::Admin::CoreContract {},
                label: "governance module 2".to_string(),
            },
        ],
        initial_items: None,
    };

    let core_addr = app
        .instantiate_contract(
            v1_core_id,
            Addr::unchecked(CREATOR_ADDR),
            &v1_core_instantiate,
            &[],
            "cw-governance",
            Some(CREATOR_ADDR.to_string()),
        )
        .unwrap();

    app.execute(
        Addr::unchecked(CREATOR_ADDR),
        CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: core_addr.to_string(),
            new_code_id: core_id,
            msg: to_json_binary(&MigrateMsg::FromV1 {
                dao_uri: None,
                params: None,
            })
            .unwrap(),
        }),
    )
    .unwrap();

    let new_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(&core_addr, &QueryMsg::DumpState {})
        .unwrap();

    let proposal_modules = new_state.proposal_modules;
    assert_eq!(2, proposal_modules.len());
    for (idx, module) in proposal_modules.iter().enumerate() {
        let prefix = derive_proposal_module_prefix(idx).unwrap();
        assert_eq!(prefix, module.prefix);
        assert_eq!(ProposalModuleStatus::Enabled, module.status);
    }

    // Check that we may not migrate more than once.
    let err: ContractError = app
        .execute(
            Addr::unchecked(CREATOR_ADDR),
            CosmosMsg::Wasm(WasmMsg::Migrate {
                contract_addr: core_addr.to_string(),
                new_code_id: core_id,
                msg: to_json_binary(&MigrateMsg::FromV1 {
                    dao_uri: None,
                    params: None,
                })
                .unwrap(),
            }),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::AlreadyMigrated {})
}

#[test]
fn test_migrate_mock() {
    let mut deps = mock_dependencies();
    let dao_uri: String = "/dao/uri".to_string();
    let msg = MigrateMsg::FromV1 {
        dao_uri: Some(dao_uri.clone()),
        params: None,
    };
    let env = mock_env();

    // Set starting version to v1.
    set_contract_version(&mut deps.storage, CONTRACT_NAME, "0.1.0").unwrap();

    // Write to storage in old proposal module format
    let proposal_modules_key = Addr::unchecked("addr");
    let old_map: Map<Addr, Empty> = Map::new("proposal_modules");
    let path = old_map.key(proposal_modules_key.clone());
    deps.storage.set(&path, &to_json_binary(&Empty {}).unwrap());

    // Write to storage in old config format
    #[cw_serde]
    struct V1Config {
        pub name: String,
        pub description: String,
        pub image_url: Option<String>,
        pub automatically_add_cw20s: bool,
        pub automatically_add_cw721s: bool,
    }

    let v1_config = V1Config {
        name: "core dao".to_string(),
        description: "a dao".to_string(),
        image_url: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
    };

    let config_item: Item<V1Config> = Item::new("config");
    config_item.save(&mut deps.storage, &v1_config).unwrap();

    // Migrate to v2
    migrate(deps.as_mut(), env, msg).unwrap();

    let new_path = PROPOSAL_MODULES.key(proposal_modules_key);
    let prop_module_bytes = deps.storage.get(&new_path).unwrap();
    let module: ProposalModule = from_json(prop_module_bytes).unwrap();
    assert_eq!(module.address, Addr::unchecked("addr"));
    assert_eq!(module.prefix, derive_proposal_module_prefix(0).unwrap());
    assert_eq!(module.status, ProposalModuleStatus::Enabled {});

    let v2_config_item: Item<Config> = Item::new("config_v2");
    let v2_config = v2_config_item.load(&deps.storage).unwrap();
    assert_eq!(v2_config.dao_uri, Some(dao_uri));
    assert_eq!(v2_config.name, v1_config.name);
    assert_eq!(v2_config.description, v1_config.description);
    assert_eq!(v2_config.image_url, v1_config.image_url);
    assert_eq!(
        v2_config.automatically_add_cw20s,
        v1_config.automatically_add_cw20s
    );
    assert_eq!(
        v2_config.automatically_add_cw721s,
        v1_config.automatically_add_cw721s
    )
}

#[test]
fn test_execute_stargate_msg() {
    let (core_addr, mut app) = do_standard_instantiate(true, None);
    let proposal_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(proposal_modules.len(), 1);
    let proposal_module = proposal_modules.into_iter().next().unwrap();

    let res = app.execute_contract(
        proposal_module.address,
        core_addr,
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![CosmosMsg::Stargate {
                type_url: "foo_type".to_string(),
                value: to_json_binary("foo_bin").unwrap(),
            }],
        },
        &[],
    );
    // TODO: Once cw-multi-test supports executing stargate/ibc messages we can change this test assert
    assert!(res.is_err());
}

#[test]
fn test_module_prefixes() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_json_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![
            ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_json_binary(&govmod_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "proposal module 1".to_string(),
            },
            ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_json_binary(&govmod_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "proposal module 2".to_string(),
            },
            ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_json_binary(&govmod_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "proposal module 2".to_string(),
            },
        ],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR_ADDR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 3);

    let module_1 = &modules[0];
    assert_eq!(module_1.status, ProposalModuleStatus::Enabled {});
    assert_eq!(module_1.prefix, "A");
    assert_eq!(&module_1.address, &modules[0].address);

    let module_2 = &modules[1];
    assert_eq!(module_2.status, ProposalModuleStatus::Enabled {});
    assert_eq!(module_2.prefix, "B");
    assert_eq!(&module_2.address, &modules[1].address);

    let module_3 = &modules[2];
    assert_eq!(module_3.status, ProposalModuleStatus::Enabled {});
    assert_eq!(module_3.prefix, "C");
    assert_eq!(&module_3.address, &modules[2].address);
}

fn get_active_modules(app: &App, gov_addr: Addr) -> Vec<ProposalModule> {
    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    modules
        .into_iter()
        .filter(|module: &ProposalModule| module.status == ProposalModuleStatus::Enabled)
        .collect()
}

fn query_proposal_module_count(app: &App, core_addr: &Addr) -> ProposalModuleCountResponse {
    app.wrap()
        .query_wasm_smart(core_addr, &QueryMsg::ProposalModuleCount {})
        .unwrap()
}

#[test]
fn test_add_remove_subdaos() {
    let (core_addr, mut app) = do_standard_instantiate(false, None);

    test_unauthorized(
        &mut app,
        core_addr.clone(),
        ExecuteMsg::UpdateSubDaos {
            to_add: vec![],
            to_remove: vec![],
        },
    );

    let to_add: Vec<SubDao> = vec![
        SubDao {
            addr: "subdao001".to_string(),
            charter: None,
        },
        SubDao {
            addr: "subdao002".to_string(),
            charter: Some("cool charter bro".to_string()),
        },
        SubDao {
            addr: "subdao005".to_string(),
            charter: None,
        },
        SubDao {
            addr: "subdao007".to_string(),
            charter: None,
        },
    ];
    let to_remove: Vec<String> = vec![];

    app.execute_contract(
        Addr::unchecked(core_addr.clone()),
        core_addr.clone(),
        &ExecuteMsg::UpdateSubDaos { to_add, to_remove },
        &[],
    )
    .unwrap();

    let res: Vec<SubDao> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ListSubDaos {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.len(), 4);

    let to_remove: Vec<String> = vec!["subdao005".to_string()];

    app.execute_contract(
        Addr::unchecked(core_addr.clone()),
        core_addr.clone(),
        &ExecuteMsg::UpdateSubDaos {
            to_add: vec![],
            to_remove,
        },
        &[],
    )
    .unwrap();

    let res: Vec<SubDao> = app
        .wrap()
        .query_wasm_smart(
            core_addr,
            &QueryMsg::ListSubDaos {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.len(), 3);

    let test_res: SubDao = SubDao {
        addr: "subdao002".to_string(),
        charter: Some("cool charter bro".to_string()),
    };

    assert_eq!(res[1], test_res);

    let full_result_set: Vec<SubDao> = vec![
        SubDao {
            addr: "subdao001".to_string(),
            charter: None,
        },
        SubDao {
            addr: "subdao002".to_string(),
            charter: Some("cool charter bro".to_string()),
        },
        SubDao {
            addr: "subdao007".to_string(),
            charter: None,
        },
    ];

    assert_eq!(res, full_result_set);
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg::FromCompatible {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}

#[test]
fn test_query_info() {
    let (core_addr, app) = do_standard_instantiate(true, None);
    let res: InfoResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &QueryMsg::Info {})
        .unwrap();
    assert_eq!(
        res,
        InfoResponse {
            info: ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string()
            }
        }
    )
}
