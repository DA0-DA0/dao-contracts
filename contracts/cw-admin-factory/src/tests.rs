use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, Empty, Reply, SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
};

use cw_core_interface::{Admin, ModuleInstantiateInfo};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::{
    contract::instantiate,
    contract::{migrate, reply, CONTRACT_NAME, CONTRACT_VERSION, INSTANTIATE_CONTRACT_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg},
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
        cw_core::contract::execute,
        cw_core::contract::instantiate,
        cw_core::contract::query,
    )
    .with_reply(cw_core::contract::reply)
    .with_migrate(cw_core::contract::migrate);
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

    let instantiate = InstantiateMsg {};
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
    let instantiate_core = cw_core::msg::InstantiateMsg {
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

    // multi-test does not support UpdateAdmin yet :(
    // https://github.com/CosmWasm/cw-plus/blob/14f4e922fac9e2097a8efa99e5b71d04747e340a/packages/multi-test/src/wasm.rs#L477
    let err = app
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
        .unwrap_err();

    assert!(matches!(
        err.downcast().unwrap(),
        cw_multi_test::error::Error::UnsupportedWasmMsg(_)
    ))
}

#[test]
pub fn test_set_admin_mock() {
    let mut deps = mock_dependencies();
    // Instantiate factory contract
    let instantiate_msg = InstantiateMsg {};
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
