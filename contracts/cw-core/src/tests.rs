use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, Uint128, WasmMsg};
use cw2::ContractVersion;
use cw_core_interface::voting::VotingPowerAtHeightResponse;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::{Duration, Expiration};

use crate::{
    msg::{
        Admin, ExecuteMsg, InitialItem, InstantiateMsg, MigrateMsg, ModuleInstantiateInfo, QueryMsg,
    },
    query::{Cw20BalanceResponse, DumpStateResponse, GetItemResponse, PauseInfoResponse},
    state::Config,
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
        cw_proposal_sudo::contract::execute,
        cw_proposal_sudo::contract::instantiate,
        cw_proposal_sudo::contract::query,
    );
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
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_id,
            msg: to_binary(&cw20_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: (0..n)
            .map(|n| ModuleInstantiateInfo {
                code_id: cw20_id,
                msg: to_binary(&cw20_instantiate).unwrap(),
                admin: Admin::CoreContract {},
                label: format!("governance module {}", n),
            })
            .collect(),
        initial_items: None,
    };
    let gov_addr = instantiate_gov(&mut app, gov_id, instantiate);

    let state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(&gov_addr, &QueryMsg::DumpState {})
        .unwrap();

    assert_eq!(
        state.config,
        Config {
            name: "DAO DAO".to_string(),
            description: "A DAO that builds DAOs.".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
        }
    );

    assert_eq!(
        state.version,
        ContractVersion {
            contract: "crates.io:cw-core".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        }
    );

    assert_eq!(state.proposal_modules.len(), n);
}

#[test]
#[should_panic(expected = "Execution would result in no governance modules being present.")]
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
            msg: to_binary(&cw20_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: format!("governance module {}", n),
        })
        .collect::<Vec<_>>();
    governance_modules.push(ModuleInstantiateInfo {
        code_id: cw20_id,
        msg: to_binary("bad").unwrap(),
        admin: Admin::CoreContract {},
        label: "I have a bad instantiate message".to_string(),
    });
    governance_modules.push(ModuleInstantiateInfo {
        code_id: cw20_id,
        msg: to_binary(&cw20_instantiate).unwrap(),
        admin: Admin::CoreContract {},
        label: "Everybody knowing
that goodness is good
makes wickedness."
            .to_string(),
    });

    let instantiate = InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_id,
            msg: to_binary(&cw20_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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

    let modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
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
    };

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        modules[0].clone(),
        &cw_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateConfig {
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
        .query_wasm_smart(gov_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(expected_config, config)
}

fn test_swap_governance(swaps: Vec<(u64, u64)>) {
    let mut app = App::default();
    let propmod_id = app.store_code(sudo_proposal_contract());
    let core_id = app.store_code(cw_core_contract());

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: propmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: propmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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

    let modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    for (add, remove) in swaps {
        let start_modules: Vec<Addr> = app
            .wrap()
            .query_wasm_smart(
                gov_addr.clone(),
                &QueryMsg::ProposalModules {
                    start_at: None,
                    limit: None,
                },
            )
            .unwrap();

        let to_add: Vec<_> = (0..add)
            .map(|n| ModuleInstantiateInfo {
                code_id: propmod_id,
                msg: to_binary(&govmod_instantiate).unwrap(),
                admin: Admin::CoreContract {},
                label: format!("governance module {}", n),
            })
            .collect();

        let to_remove: Vec<_> = start_modules
            .iter()
            .rev()
            .take(remove as usize)
            .map(|a| a.to_string())
            .collect();

        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            start_modules[0].clone(),
            &cw_proposal_sudo::msg::ExecuteMsg::Execute {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: gov_addr.to_string(),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::UpdateProposalModules { to_add, to_remove })
                        .unwrap(),
                }
                .into()],
            },
            &[],
        )
        .unwrap();

        let finish_modules: Vec<Addr> = app
            .wrap()
            .query_wasm_smart(
                gov_addr.clone(),
                &QueryMsg::ProposalModules {
                    start_at: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(
            finish_modules.len() as u64,
            start_modules.len() as u64 + add - remove
        );
        for module in start_modules.into_iter().rev().take(remove as usize) {
            assert!(!finish_modules.contains(&module))
        }
    }
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
#[should_panic(expected = "Execution would result in no governance modules being present.")]
fn test_swap_governance_bad() {
    test_swap_governance(vec![(1, 1), (0, 1)])
}

#[test]
fn test_removed_modules_can_not_execute() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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

    let modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    let start_module = modules.into_iter().next().unwrap();

    let to_add = vec![ModuleInstantiateInfo {
        code_id: govmod_id,
        msg: to_binary(&govmod_instantiate).unwrap(),
        admin: Admin::CoreContract {},
        label: "new governance module".to_string(),
    }];

    let to_remove = vec![start_module.to_string()];

    // Swap ourselves out.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        start_module.clone(),
        &cw_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateProposalModules { to_add, to_remove }).unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    let finish_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    let new_proposal_module = finish_modules.into_iter().next().unwrap();

    // Try to add a new module and remove the one we added
    // earlier. This should fail as we have been removed.
    let to_add = vec![ModuleInstantiateInfo {
        code_id: govmod_id,
        msg: to_binary(&govmod_instantiate).unwrap(),
        admin: Admin::CoreContract {},
        label: "new governance module".to_string(),
    }];
    let to_remove = vec![new_proposal_module.to_string()];

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            start_module,
            &cw_proposal_sudo::msg::ExecuteMsg::Execute {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: gov_addr.to_string(),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::UpdateProposalModules {
                        to_add: to_add.clone(),
                        to_remove: to_remove.clone(),
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
    assert!(matches!(err, ContractError::Unauthorized {}));

    // The new proposal module should be able to perform actions.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        new_proposal_module,
        &cw_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateProposalModules { to_add, to_remove }).unwrap(),
            }
            .into()],
        },
        &[],
    )
    .unwrap();
}

#[test]
fn test_swap_voting_module() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let gov_id = app.store_code(cw_core_contract());

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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

    let modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(modules.len(), 1);

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        modules[0].clone(),
        &cw_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateVotingModule {
                    module: ModuleInstantiateInfo {
                        code_id: govmod_id,
                        msg: to_binary(&govmod_instantiate).unwrap(),
                        admin: Admin::CoreContract {},
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

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };

    let gov_instantiate = InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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
                msg: to_binary(&govmod_instantiate).unwrap(),
                admin: Admin::CoreContract {},
                label: "voting module".to_string(),
            },
        },
    );

    test_unauthorized(
        &mut app,
        gov_addr.clone(),
        ExecuteMsg::UpdateProposalModules {
            to_add: vec![],
            to_remove: vec![],
        },
    );

    test_unauthorized(
        &mut app,
        gov_addr,
        ExecuteMsg::UpdateConfig {
            config: Config {
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

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = cw20_balance_voting::msg::InstantiateMsg {
        token_info: cw20_balance_voting::msg::TokenInfo::New {
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
        admin,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: auto_add,
        automatically_add_cw721s: auto_add,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_binary(&voting_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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
    let proposal_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
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
                msg: to_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(res.is_err());

    // Proposal mdoule can't call ExecuteAdminMsgs
    let res = app.execute_contract(
        proposal_module.clone(),
        core_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(res.is_err());

    // Update Admin can't be called by non-admins
    let res = app.execute_contract(
        Addr::unchecked("rando"),
        core_addr.clone(),
        &ExecuteMsg::UpdateAdmin {
            admin: Some(Addr::unchecked("rando")),
        },
        &[],
    );
    assert!(res.is_err());

    // Update Admin can't be called, even by the core module
    let res = app.execute_contract(
        core_addr.clone(),
        core_addr.clone(),
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_binary(&ExecuteMsg::UpdateAdmin {
                    admin: Some(Addr::unchecked("meow")),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(res.is_err());

    // Instantiate new DAO with an admin
    let (core_with_admin_addr, mut app) =
        do_standard_instantiate(true, Some(Addr::unchecked("admin").to_string()));

    // Non admins still can't call ExecuteAdminMsgs
    let res = app.execute_contract(
        proposal_module,
        core_with_admin_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_with_admin_addr.to_string(),
                msg: to_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(res.is_err());

    // Admin can call ExecuteAdminMsgs, here an admin pasues the DAO
    let res = app.execute_contract(
        Addr::unchecked("admin"),
        core_with_admin_addr.clone(),
        &ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_with_admin_addr.to_string(),
                msg: to_binary(&ExecuteMsg::Pause {
                    duration: Duration::Height(10),
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(res.is_ok());

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
    app.update_block(|mut block| block.height += 11);

    // Admin can update the admin.
    let res = app.execute_contract(
        Addr::unchecked("admin"),
        core_with_admin_addr.clone(),
        &ExecuteMsg::UpdateAdmin {
            admin: Some(Addr::unchecked("meow")),
        },
        &[],
    );
    assert!(res.is_ok());

    // Check that admin has been updated
    let res: Option<Addr> = app
        .wrap()
        .query_wasm_smart(core_with_admin_addr, &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(res, Some(Addr::unchecked("meow")));
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

fn set_item(app: &mut App, gov_addr: Addr, key: String, addr: String) {
    app.execute_contract(
        gov_addr.clone(),
        gov_addr,
        &ExecuteMsg::SetItem { key, addr },
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
        .query_wasm_smart(gov_addr, &QueryMsg::ListItems { start_at, limit })
        .unwrap()
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

    remove_item(&mut app, gov_addr.clone(), "a".to_string());
    let a = get_item(&mut app, gov_addr.clone(), "a".to_string());
    assert_eq!(a, GetItemResponse { item: None });

    remove_item(&mut app, gov_addr, "b".to_string());
}

#[test]
fn test_list_items() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = cw20_balance_voting::msg::InstantiateMsg {
        token_info: cw20_balance_voting::msg::TokenInfo::New {
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
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_binary(&voting_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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

    // Foo returned as we are only getting one item and items are in
    // decending order.
    let first_item = list_items(&mut app, gov_addr.clone(), None, Some(1));
    assert_eq!(first_item.len(), 1);
    assert_eq!(first_item[0], ("fookey".to_string(), "fooaddr".to_string()));

    let no_items = list_items(&mut app, gov_addr.clone(), None, Some(0));
    assert_eq!(no_items.len(), 0);

    // Items are retreived in decending order so asking for foo with
    // no limit ought to give us a single item.
    let second_item = list_items(&mut app, gov_addr, Some("foo".to_string()), None);
    assert_eq!(second_item.len(), 1);
    assert_eq!(
        second_item[0],
        ("fookey".to_string(), "fooaddr".to_string())
    );
}

#[test]
fn test_instantiate_with_items() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = cw20_balance_voting::msg::InstantiateMsg {
        token_info: cw20_balance_voting::msg::TokenInfo::New {
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
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_binary(&voting_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "governance module".to_string(),
        }],
        initial_items: Some(vec![
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
        ]),
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

    // Ensure initial items were added. One was overriden.
    let items = list_items(&mut app, gov_addr.clone(), None, None);
    assert_eq!(items.len(), 2);

    // Descending order, so item1 is first.
    assert_eq!(items[1].0, "item0".to_string());
    let get_item0 = get_item(&mut app, gov_addr.clone(), "item0".to_string());
    assert_eq!(
        get_item0,
        GetItemResponse {
            item: Some("item0_value_override".to_string()),
        }
    );

    assert_eq!(items[0].0, "item1".to_string());
    let item1_value = get_item(&mut app, gov_addr, "item1".to_string()).item;
    assert_eq!(item1_value, Some("item1_value".to_string()))
}

#[test]
fn test_cw20_receive_auto_add() {
    let (gov_addr, mut app) = do_standard_instantiate(true, None);

    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(gov_addr.clone(), &QueryMsg::VotingModule {})
        .unwrap();
    let gov_token: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cw_core_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Check that the balances query works with no tokens.
    let cw20_balances: Vec<Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20Balances {
                start_at: None,
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
            msg: to_binary(&"").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20TokenList {
                start_at: None,
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
                start_at: None,
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

    // Test removing and adding some new ones.
    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw20List {
            to_add: vec!["new".to_string()],
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
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![Addr::unchecked("new")]);
}

#[test]
fn test_cw20_receive_no_auto_add() {
    let (gov_addr, mut app) = do_standard_instantiate(false, None);

    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(gov_addr.clone(), &QueryMsg::VotingModule {})
        .unwrap();
    let gov_token: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cw_core_interface::voting::Query::TokenContract {},
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
            msg: to_binary(&"").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw20_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw20TokenList {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, Vec::<Addr>::new());

    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw20List {
            to_add: vec!["new".to_string(), gov_token.to_string()],
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
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![Addr::unchecked("new"), gov_token]);
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

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id: "ekez".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::<Option<Empty>>::SendNft {
            contract: gov_addr.to_string(),
            token_id: "ekez".to_string(),
            msg: to_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw721_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw721TokenList {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw721_list, vec![cw721_addr.clone()]);

    // OK to add already added. Remove happens after add.
    app.execute_contract(
        Addr::unchecked(gov_addr.clone()),
        gov_addr.clone(),
        &ExecuteMsg::UpdateCw721List {
            to_add: vec!["new".to_string(), cw721_addr.to_string()],
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
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![Addr::unchecked("new")]);
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

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id: "ekez".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::msg::ExecuteMsg::<Option<Empty>>::SendNft {
            contract: gov_addr.to_string(),
            token_id: "ekez".to_string(),
            msg: to_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    let cw721_list: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &QueryMsg::Cw721TokenList {
                start_at: None,
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
                "new".to_string(),
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
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(cw20_list, vec![Addr::unchecked("new"), cw721_addr]);
}

#[test]
fn test_pause() {
    let (core_addr, mut app) = do_standard_instantiate(false, None);

    let start_height = app.block_info().height;

    let proposal_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &QueryMsg::ProposalModules {
                start_at: None,
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
            proposal_module.clone(),
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
        proposal_module.clone(),
        core_addr.clone(),
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_binary(&ExecuteMsg::Pause {
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
            proposal_module.clone(),
            core_addr.clone(),
            &ExecuteMsg::ExecuteProposalHook {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: core_addr.to_string(),
                    msg: to_binary(&ExecuteMsg::Pause {
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

    app.update_block(|mut block| block.height += 9);

    // Still not unpaused.
    let err: ContractError = app
        .execute_contract(
            proposal_module.clone(),
            core_addr.clone(),
            &ExecuteMsg::ExecuteProposalHook {
                msgs: vec![WasmMsg::Execute {
                    contract_addr: core_addr.to_string(),
                    msg: to_binary(&ExecuteMsg::Pause {
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

    app.update_block(|mut block| block.height += 1);

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
        proposal_module,
        core_addr.clone(),
        &ExecuteMsg::ExecuteProposalHook {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_binary(&ExecuteMsg::Pause {
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
fn test_migrate() {
    let mut app = App::default();
    let govmod_id = app.store_code(sudo_proposal_contract());
    let voting_id = app.store_code(cw20_balances_voting());
    let gov_id = app.store_code(cw_core_contract());
    let cw20_id = app.store_code(cw20_contract());

    let govmod_instantiate = cw_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR_ADDR.to_string(),
    };
    let voting_instantiate = cw20_balance_voting::msg::InstantiateMsg {
        token_info: cw20_balance_voting::msg::TokenInfo::New {
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
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_binary(&voting_instantiate).unwrap(),
            admin: Admin::CoreContract {},
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Admin::CoreContract {},
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
            msg: to_binary(&MigrateMsg {}).unwrap(),
        }),
    )
    .unwrap();

    let new_state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &QueryMsg::DumpState {})
        .unwrap();

    assert_eq!(new_state, state);
}
