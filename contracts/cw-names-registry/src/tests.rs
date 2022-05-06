use crate::msg::{InstantiateMsg, LookUpNameResponse, QueryMsg, ReceiveMsg};
use anyhow::Result as AnyResult;
use cosmwasm_std::{to_binary, Addr, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

const DAO_ADDR: &str = "dao";
const ADMIN_ADDR: &str = "admin";
const NON_ADMIN_ADDR: &str = "nonadmin";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn names_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn create_token(app: &mut App) -> Addr {
    let cw20_id = app.store_code(cw20_contract());
    app.instantiate_contract(
        cw20_id,
        Addr::unchecked(ADMIN_ADDR),
        &cw20_base::msg::InstantiateMsg {
            name: "Name Registry Token".to_string(),
            symbol: "NAME".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: DAO_ADDR.to_string(),
                    amount: Uint128::new(1000),
                },
                Cw20Coin {
                    address: ADMIN_ADDR.to_string(),
                    amount: Uint128::new(1000),
                },
                Cw20Coin {
                    address: NON_ADMIN_ADDR.to_string(),
                    amount: Uint128::new(1000),
                },
            ],
            mint: None,
            marketing: None,
        },
        &[],
        "some token",
        None,
    )
    .unwrap()
}

fn setup_test_case(app: &mut App, payment_amount: Uint128) -> (Addr, Addr) {
    let names_id = app.store_code(names_contract());

    let token_addr = create_token(app);

    let names_addr = app
        .instantiate_contract(
            names_id,
            Addr::unchecked(ADMIN_ADDR),
            &InstantiateMsg {
                admin: ADMIN_ADDR.to_string(),
                payment_token_address: token_addr.to_string(),
                payment_amount,
            },
            &[],
            "DAO Names Registry",
            None,
        )
        .unwrap();

    (names_addr, token_addr)
}

#[test]
fn test_instantiate() {
    let mut app = App::default();
    let (_names, _token) = setup_test_case(&mut app, Uint128::new(50));
}

fn register(
    app: &mut App,
    names_addr: Addr,
    amount: Uint128,
    name: String,
    sender: Addr,
    token_addr: Addr,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: names_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::Register { name }).unwrap(),
    };
    app.execute_contract(sender, token_addr, &msg, &[])
}

fn query_name(app: &mut App, names_addr: Addr, name: String) -> LookUpNameResponse {
    let msg = QueryMsg::LookUpName { name };
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

#[test]
fn test_register() {
    let mut app = App::default();
    let (names, token) = setup_test_case(&mut app, Uint128::new(50));
    let other_token = create_token(&mut app); // To be used when sending wrong token
    let name: &str = "Name";

    // Send wrong token
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        other_token,
    )
    .unwrap_err();

    // Send too little
    register(
        &mut app,
        names.clone(),
        Uint128::new(25),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        token.clone(),
    )
    .unwrap_err();

    // Valid
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        token.clone(),
    )
    .unwrap();

    let resp = query_name(&mut app, names.clone(), name.to_string());
    assert!(resp.dao.is_some());
    assert_eq!(resp.dao, Some(Addr::unchecked(DAO_ADDR)));

    // Name already taken
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
        token.clone(),
    )
    .unwrap_err();

    // DAO already has name
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        "Name2".to_string(),
        Addr::unchecked(DAO_ADDR),
        token,
    )
    .unwrap_err();

    // Name has not been registered
    let resp = query_name(&mut app, names, "Name2".to_string());
    assert!(resp.dao.is_none());
    assert_eq!(resp.dao, None);
}

#[test]
fn test_revoke() {}

#[test]
fn test_update_config() {}
