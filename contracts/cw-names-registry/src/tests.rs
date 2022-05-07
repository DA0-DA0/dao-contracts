use crate::msg::{
    ExecuteMsg, InstantiateMsg, LookUpDaoResponse, LookUpNameResponse, QueryMsg, ReceiveMsg,
};
use crate::state::Config;
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

fn revoke(app: &mut App, names_addr: Addr, name: String, sender: Addr) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Revoke { name };
    app.execute_contract(sender, names_addr, &msg, &[])
}

fn reserve(app: &mut App, names_addr: Addr, name: String, sender: Addr) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Reserve { name };
    app.execute_contract(sender, names_addr, &msg, &[])
}

fn transfer_reservation(
    app: &mut App,
    names_addr: Addr,
    name: String,
    dao: String,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::TransferReservation { name, dao };
    app.execute_contract(sender, names_addr, &msg, &[])
}

fn update_config(
    app: &mut App,
    names_addr: Addr,
    new_payment_token_address: Option<String>,
    new_admin: Option<String>,
    new_payment_amount: Option<Uint128>,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig {
        new_payment_token_address,
        new_admin,
        new_payment_amount,
    };
    app.execute_contract(sender, names_addr, &msg, &[])
}

fn query_name(app: &mut App, names_addr: Addr, name: String) -> LookUpNameResponse {
    let msg = QueryMsg::LookUpName { name };
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

fn query_dao(app: &mut App, names_addr: Addr, dao: String) -> LookUpDaoResponse {
    let msg = QueryMsg::LookUpDao { dao };
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

fn query_config(app: &mut App, names_addr: Addr) -> Config {
    let msg = QueryMsg::Config {};
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

    // Reserve a name to test failure to register reserved name
    reserve(
        &mut app,
        names.clone(),
        "Reserved".to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Try to register a reserved name
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        "Reserved".to_string(),
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

    // Look it up both ways
    let resp = query_name(&mut app, names.clone(), name.to_string());
    assert!(resp.dao.is_some());
    assert!(!resp.reserved);
    assert_eq!(resp.dao, Some(Addr::unchecked(DAO_ADDR)));
    let resp = query_dao(&mut app, names.clone(), DAO_ADDR.to_string());
    assert!(resp.name.is_some());
    assert_eq!(resp.name, Some(name.to_string()));

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
    assert!(!resp.reserved);
    assert!(resp.dao.is_none());
    assert_eq!(resp.dao, None);
}

#[test]
fn test_revoke() {
    let mut app = App::default();
    let (names, token) = setup_test_case(&mut app, Uint128::new(50));
    let name: &str = "Name";

    // Register the name
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        token.clone(),
    )
    .unwrap();

    // Try to revoke non existent name, will fail
    revoke(
        &mut app,
        names.clone(),
        "NotExist".to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap_err();

    // Try to revoke as non admin and not DAO, will fail
    revoke(
        &mut app,
        names.clone(),
        name.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // Revoke as owner, will succeed
    revoke(
        &mut app,
        names.clone(),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap();

    // Reregister to test revoking as admin
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        token,
    )
    .unwrap();

    // Revoke as admin, will succeed
    revoke(
        &mut app,
        names,
        name.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();
}

#[test]
fn test_reserve() {
    let mut app = App::default();
    let (names, token) = setup_test_case(&mut app, Uint128::new(50));
    let name: &str = "Name";
    let already_registered_name: &str = "Already";

    // Register this name
    register(
        &mut app,
        names.clone(),
        Uint128::new(50),
        already_registered_name.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
        token,
    )
    .unwrap();

    // Try to reserve registered name, will fail
    reserve(
        &mut app,
        names.clone(),
        already_registered_name.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap_err();

    // Try to reserve as non admin, will fail
    reserve(
        &mut app,
        names.clone(),
        name.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    let res = query_name(&mut app, names.clone(), name.to_string());
    assert!(!res.reserved);

    // Reserve a name
    reserve(
        &mut app,
        names.clone(),
        name.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let res = query_name(&mut app, names.clone(), name.to_string());
    assert!(res.reserved);

    // Try to transfer as non admin
    transfer_reservation(
        &mut app,
        names.clone(),
        name.to_string(),
        DAO_ADDR.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // Try to transfer unreserved name
    transfer_reservation(
        &mut app,
        names.clone(),
        "NotReserved".to_string(),
        DAO_ADDR.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap_err();

    // Try to transfer to a DAO that already has a name
    transfer_reservation(
        &mut app,
        names.clone(),
        name.to_string(),
        NON_ADMIN_ADDR.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap_err();

    // Successfully transfer
    transfer_reservation(
        &mut app,
        names.clone(),
        name.to_string(),
        DAO_ADDR.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let res = query_name(&mut app, names.clone(), name.to_string());
    assert!(!res.reserved);
    assert_eq!(res.dao, Some(Addr::unchecked(DAO_ADDR)));

    // Try to reserve the newly registered name from transfer, will fail
    reserve(
        &mut app,
        names,
        name.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap_err();
}

#[test]
fn test_update_config() {
    let mut app = App::default();
    let (names, token) = setup_test_case(&mut app, Uint128::new(50));
    let other_token = create_token(&mut app); // To be used when updating payment token

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR),
            payment_token_address: token,
            payment_amount: Uint128::new(50),
        }
    );

    // Update config as non admin fails
    update_config(
        &mut app,
        names.clone(),
        Some(other_token.to_string()),
        Some(NON_ADMIN_ADDR.to_string()),
        Some(Uint128::new(25)),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // Update config as admin
    update_config(
        &mut app,
        names.clone(),
        Some(other_token.to_string()),
        Some(NON_ADMIN_ADDR.to_string()),
        Some(Uint128::new(25)),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names);
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(NON_ADMIN_ADDR),
            payment_token_address: other_token,
            payment_amount: Uint128::new(25),
        }
    );
}
