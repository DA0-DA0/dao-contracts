use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, Empty, Reply, SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
};

use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use dao_interface::{Admin, ModuleInstantiateInfo};

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
        dao_core::contract::execute,
        dao_core::contract::instantiate,
        dao_core::contract::query,
    )
    .with_reply(dao_core::contract::reply)
    .with_migrate(dao_core::contract::migrate);
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

    // // Get the core address from the instantiate event
    // let instantiate_event = &res.events[2];
    // assert_eq!(instantiate_event.ty, "instantiate");
    // let core_addr = instantiate_event.attributes[0].value.clone();

    // // Check that admin of core address is itself
    // let contract_info = app.wrap().query_wasm_contract_info(&core_addr).unwrap();
    // assert_eq!(contract_info.admin, Some(core_addr))
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
