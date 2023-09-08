use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, Coin, Empty, Reply, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
    WasmMsg,
};

use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use dao_interface::state::{Admin, ModuleInstantiateInfo};

use crate::{
    contract::instantiate,
    contract::{migrate, reply, CONTRACT_NAME, CONTRACT_VERSION, INSTANTIATE_CONTRACT_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};

fn factory_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn cw_core_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_dao_core::contract::execute,
        dao_dao_core::contract::instantiate,
        dao_dao_core::contract::query,
    )
    .with_reply(dao_dao_core::contract::reply)
    .with_migrate(dao_dao_core::contract::migrate);
    Box::new(contract)
}

#[test]
pub fn test_set_admin() {
    let mut app = App::default();
    let code_id = app.store_code(factory_contract());
    let cw20_code_id = app.store_code(cw20_contract());
    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "DAO".to_string(),
        symbol: "DAO".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
    };

    let instantiate = InstantiateMsg {
        fee: None,
        owner: None,
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Instantiate core contract using factory.
    let cw_core_code_id = app.store_code(cw_core_contract());
    let instantiate_core = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_code_id,
            msg: to_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                label: "prop module".to_string(),
            },
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                label: "prop module 2".to_string(),
            },
        ],
        initial_items: None,
    };

    let res: AppResponse = app
        .execute_contract(
            Addr::unchecked("CREATOR"),
            factory_addr,
            &ExecuteMsg::InstantiateContractWithSelfAdmin {
                instantiate_msg: to_binary(&instantiate_core).unwrap(),
                code_id: cw_core_code_id,
                label: "my contract".to_string(),
            },
            &[],
        )
        .unwrap();

    // Get the core address from the instantiate event
    let instantiate_event = &res.events[2];
    assert_eq!(instantiate_event.ty, "instantiate");
    let core_addr = instantiate_event.attributes[0].value.clone();

    // Check that admin of core address is itself
    let contract_info = app.wrap().query_wasm_contract_info(&core_addr).unwrap();
    assert_eq!(contract_info.admin, Some(core_addr))
}

#[test]
pub fn test_set_admin_mock() {
    let mut deps = mock_dependencies();
    // Instantiate factory contract
    let instantiate_msg = InstantiateMsg {
        fee: None,
        owner: None,
    };
    let info = mock_info("creator", &[]);
    let env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
    let bytes = vec![10, 9, 99, 111, 110, 116, 114, 97, 99, 116, 50];
    let reply_msg: Reply = Reply {
        id: INSTANTIATE_CONTRACT_REPLY_ID,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: (Some(Binary(bytes))),
        }),
    };

    let res = reply(deps.as_mut(), env, reply_msg).unwrap();
    assert_eq!(res.attributes.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(WasmMsg::UpdateAdmin {
            contract_addr: "contract2".to_string(),
            admin: "contract2".to_string()
        })
    )
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}

#[test]
pub fn test_fee() {
    let creator = Addr::unchecked("alice");
    let user = Addr::unchecked("bob");
    let balance = vec![Coin {
        denom: "juno".to_string(),
        amount: Uint128::from(100u128),
    }];
    // Start with a fee higher than the user's balance
    let fee = vec![Coin {
        denom: "juno".to_string(),
        amount: Uint128::from(200u128),
    }];
    let mut app = App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(storage, &user, balance.clone())
            .unwrap();
    });

    let code_id = app.store_code(factory_contract());
    let cw20_code_id = app.store_code(cw20_contract());
    let cw_core_code_id = app.store_code(cw_core_contract());

    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "DAO".to_string(),
        symbol: "DAO".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
    };

    let instantiate = InstantiateMsg {
        fee: Some(fee.clone()),
        owner: Some(creator.to_string()),
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Check that the fee was set on instantiate
    let fee_result: Option<Vec<Coin>> = app
        .wrap()
        .query_wasm_smart(factory_addr.clone(), &QueryMsg::Fee {})
        .unwrap();
    assert_eq!(fee_result, Some(fee.clone()));

    // Instantiate core contract using factory.
    let instantiate_core = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_code_id,
            msg: to_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                label: "prop module".to_string(),
            },
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                label: "prop module 2".to_string(),
            },
        ],
        initial_items: None,
    };

    // Fail on insufficient funds
    let res = app.execute_contract(
        user.clone(),
        factory_addr.clone(),
        &ExecuteMsg::InstantiateContractWithSelfAdmin {
            instantiate_msg: to_binary(&instantiate_core).unwrap(),
            code_id: cw_core_code_id,
            label: "my contract".to_string(),
        },
        &balance,
    );
    assert!(res.is_err());

    // Update fee to exact balance
    app.execute_contract(
        creator.clone(),
        factory_addr.clone(),
        &ExecuteMsg::UpdateFee {
            fee: Some(balance.clone()),
        },
        &vec![],
    )
    .unwrap();

    // Success with a fee
    let res = app.execute_contract(
        user.clone(),
        factory_addr.clone(),
        &ExecuteMsg::InstantiateContractWithSelfAdmin {
            instantiate_msg: to_binary(&instantiate_core).unwrap(),
            code_id: cw_core_code_id,
            label: "my contract".to_string(),
        },
        &balance,
    );
    assert!(res.is_ok());
    // Check that the owner received funds
    assert_eq!(
        app.wrap().query_balance(creator.clone(), "juno").unwrap(),
        balance[0]
    );

    // Remove fee
    app.execute_contract(
        creator.clone(),
        factory_addr.clone(),
        &ExecuteMsg::UpdateFee { fee: None },
        &vec![],
    )
    .unwrap();

    // Success with no fee
    let res = app.execute_contract(
        user.clone(),
        factory_addr.clone(),
        &ExecuteMsg::InstantiateContractWithSelfAdmin {
            instantiate_msg: to_binary(&instantiate_core).unwrap(),
            code_id: cw_core_code_id,
            label: "my contract".to_string(),
        },
        &vec![],
    );
    assert!(res.is_ok());

    // Fail update fee - not owner
    let res = app.execute_contract(
        user.clone(),
        factory_addr.clone(),
        &ExecuteMsg::UpdateFee {
            fee: Some(fee.clone()),
        },
        &vec![],
    );
    assert!(res.is_err());
}
