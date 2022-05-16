use crate::msg::{
    ExecuteMsg, InstantiateMsg, IsNameAvailableToRegisterResponse, LookUpDaoByNameResponse,
    LookUpNameByDaoResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, PaymentInfo};
use anyhow::Result as AnyResult;
use cosmwasm_std::{coins, to_binary, Addr, Coin, Empty, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
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

fn setup_app() -> App {
    let amount = Uint128::new(10000);
    App::new(|r, _a, s| {
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(DAO_ADDR),
                vec![
                    Coin {
                        denom: "ujuno".to_string(),
                        amount,
                    },
                    Coin {
                        denom: "uatom".to_string(),
                        amount,
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(NON_ADMIN_ADDR),
                vec![
                    Coin {
                        denom: "ujuno".to_string(),
                        amount,
                    },
                    Coin {
                        denom: "uatom".to_string(),
                        amount,
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADMIN_ADDR),
                vec![
                    Coin {
                        denom: "ujuno".to_string(),
                        amount,
                    },
                    Coin {
                        denom: "uatom".to_string(),
                        amount,
                    },
                ],
            )
            .unwrap();
    })
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

fn setup_test_case(app: &mut App, payment_info: PaymentInfo) -> Addr {
    let names_id = app.store_code(names_contract());
    app.instantiate_contract(
        names_id,
        Addr::unchecked(ADMIN_ADDR),
        &InstantiateMsg {
            admin: ADMIN_ADDR.to_string(),
            payment_info,
        },
        &[],
        "DAO Names Registry",
        None,
    )
    .unwrap()
}

#[test]
fn test_instantiate() {
    let mut app = setup_app();
    let token_addr = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token_addr.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let names_id = app.store_code(names_contract());

    let _err = app
        .instantiate_contract(
            names_id,
            Addr::unchecked(ADMIN_ADDR),
            &InstantiateMsg {
                admin: ADMIN_ADDR.to_string(),
                payment_info: PaymentInfo::Cw20Payment {
                    token_address: names.to_string(),
                    payment_amount: Uint128::new(50),
                },
            },
            &[],
            "DAO Names Registry",
            None,
        )
        .unwrap_err();
}

fn register_cw20(
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

fn register_native(
    app: &mut App,
    names_addr: Addr,
    amount: u128,
    denom: &str,
    name: String,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::RegisterName { name };
    app.execute_contract(sender, names_addr, &msg, &coins(amount, denom))
}

fn query_cw20_balance(app: &mut App, token_addr: Addr, addr: Addr) -> Uint128 {
    let msg = cw20_base::msg::QueryMsg::Balance {
        address: addr.to_string(),
    };
    let res: BalanceResponse = app.wrap().query_wasm_smart(token_addr, &msg).unwrap();
    res.balance
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
    new_admin: Option<String>,
    new_payment_info: Option<PaymentInfo>,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig {
        new_admin,
        new_payment_info,
    };
    app.execute_contract(sender, names_addr, &msg, &[])
}

fn query_name(app: &mut App, names_addr: Addr, name: String) -> LookUpDaoByNameResponse {
    let msg = QueryMsg::LookUpDaoByName { name };
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

fn query_dao(app: &mut App, names_addr: Addr, dao: String) -> LookUpNameByDaoResponse {
    let msg = QueryMsg::LookUpNameByDao { dao };
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

fn query_availability(
    app: &mut App,
    names_addr: Addr,
    name: String,
) -> IsNameAvailableToRegisterResponse {
    let msg = QueryMsg::IsNameAvailableToRegister { name };
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

fn query_config(app: &mut App, names_addr: Addr) -> Config {
    let msg = QueryMsg::Config {};
    app.wrap().query_wasm_smart(names_addr, &msg).unwrap()
}

#[test]
fn test_register_cw20() {
    let mut app = setup_app();
    let token = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let other_token = create_token(&mut app); // To be used when sending wrong token
    let name: &str = "Name";

    // Try and register using natives funds will fail
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujunox",
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap_err();

    // Send wrong token
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        other_token,
    )
    .unwrap_err();

    // Send too little
    register_cw20(
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
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        "Reserved".to_string(),
        Addr::unchecked(DAO_ADDR),
        token.clone(),
    )
    .unwrap_err();

    // Valid
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        token.clone(),
    )
    .unwrap();

    // Check the admin now has 1050
    // It started with 1000 and now has 50 from the success
    let balance = query_cw20_balance(&mut app, token.clone(), Addr::unchecked(ADMIN_ADDR));
    assert_eq!(balance, Uint128::new(1050));

    // Look it up both ways
    let resp = query_name(&mut app, names.clone(), name.to_string());
    assert!(resp.dao.is_some());
    assert_eq!(resp.dao, Some(Addr::unchecked(DAO_ADDR)));
    let resp = query_dao(&mut app, names.clone(), DAO_ADDR.to_string());
    assert!(resp.name.is_some());
    assert_eq!(resp.name, Some(name.to_string()));

    // Name already taken
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
        token.clone(),
    )
    .unwrap_err();

    // DAO already has name
    register_cw20(
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
fn test_register_native() {
    let mut app = setup_app();
    let names = setup_test_case(
        &mut app,
        PaymentInfo::NativePayment {
            token_denom: "ujuno".to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let token = create_token(&mut app);
    let name: &str = "Name";

    // Try and register with a cw20 will fail
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
        token,
    )
    .unwrap_err();

    // Send no coins
    let msg = ExecuteMsg::RegisterName {
        name: name.to_string(),
    };
    app.execute_contract(Addr::unchecked(DAO_ADDR), names.clone(), &msg, &[])
        .unwrap_err();

    // Send wrong denom
    register_native(
        &mut app,
        names.clone(),
        50,
        "uatom",
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap_err();

    // Not enough
    register_native(
        &mut app,
        names.clone(),
        25,
        "ujuno",
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap_err();

    // Reserve a name to test failure to register a reserved name
    reserve(
        &mut app,
        names.clone(),
        "Reserved".to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Try to register a reserved name
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujuno",
        "Reserved".to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap_err();

    // Valid
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujuno",
        name.to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap();

    // Check balance, should now have 10050 ujuno
    // As it started with 10000
    let coin = app.wrap().query_balance(ADMIN_ADDR, "ujuno").unwrap();
    assert_eq!(coin.amount, Uint128::new(10050));

    // Look it up both ways
    let resp = query_name(&mut app, names.clone(), name.to_string());
    assert!(resp.dao.is_some());
    assert_eq!(resp.dao, Some(Addr::unchecked(DAO_ADDR)));
    let resp = query_dao(&mut app, names.clone(), DAO_ADDR.to_string());
    assert!(resp.name.is_some());
    assert_eq!(resp.name, Some(name.to_string()));

    // Name already taken
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujuno",
        name.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // DAO already has name
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujuno",
        "Name2".to_string(),
        Addr::unchecked(DAO_ADDR),
    )
    .unwrap_err();

    // Name has not been registered
    let resp = query_name(&mut app, names, "Name2".to_string());
    assert!(resp.dao.is_none());
    assert_eq!(resp.dao, None);
}

#[test]
fn test_payment_info_switch() {
    let mut app = setup_app();
    let token = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let other_token = create_token(&mut app); // To be used in an update config call
    let name1: &str = "Name1";
    let name2: &str = "Name2";
    let name3: &str = "Name3";

    // Start with token register successfully
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name1.to_string(),
        Addr::unchecked(DAO_ADDR),
        token.clone(),
    )
    .unwrap();

    // Other token will fail
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name2.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
        other_token.clone(),
    )
    .unwrap_err();

    // Native will fail
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujuno",
        name3.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // Keep CW20 payments but switch token
    update_config(
        &mut app,
        names.clone(),
        None,
        Some(PaymentInfo::Cw20Payment {
            token_address: other_token.to_string(),
            payment_amount: Uint128::new(50),
        }),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Original token will now fail
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name3.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
        token.clone(),
    )
    .unwrap_err();

    // Other token will now succeed
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name2.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
        other_token.clone(),
    )
    .unwrap();

    // Native still fails
    register_native(
        &mut app,
        names.clone(),
        50,
        "ujuno",
        name3.to_string(),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // Now switch to native payments
    update_config(
        &mut app,
        names.clone(),
        None,
        Some(PaymentInfo::NativePayment {
            token_denom: "ujuno".to_string(),
            payment_amount: Uint128::new(50),
        }),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Original token fails
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name3.to_string(),
        Addr::unchecked(ADMIN_ADDR),
        token,
    )
    .unwrap_err();

    // Other token will now fail again
    register_cw20(
        &mut app,
        names.clone(),
        Uint128::new(50),
        name3.to_string(),
        Addr::unchecked(ADMIN_ADDR),
        other_token,
    )
    .unwrap_err();

    // Native now succeeds
    register_native(
        &mut app,
        names,
        50,
        "ujuno",
        name3.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();
}

#[test]
fn test_revoke() {
    let mut app = setup_app();
    let token = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let name: &str = "Name";

    // Register the name
    register_cw20(
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
    register_cw20(
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
    let mut app = setup_app();
    let token = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let name: &str = "Name";
    let already_registered_name: &str = "Already";

    // Register this name
    register_cw20(
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

    // Name is not reserved not taken
    let res = query_availability(&mut app, names.clone(), name.to_string());
    assert!(!res.taken);
    assert!(!res.reserved);

    // Reserve a name
    reserve(
        &mut app,
        names.clone(),
        name.to_string(),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();
    let res = query_availability(&mut app, names.clone(), name.to_string());
    assert!(!res.taken);
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
    assert_eq!(res.dao, Some(Addr::unchecked(DAO_ADDR)));
    let res = query_availability(&mut app, names.clone(), name.to_string());
    assert!(res.taken);
    assert!(!res.reserved);

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
    let mut app = setup_app();
    let token = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let other_token = create_token(&mut app); // To be used when updating payment token

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR),
            payment_info: PaymentInfo::Cw20Payment {
                token_address: token.to_string(),
                payment_amount: Uint128::new(50)
            }
        }
    );

    // Update config as non admin fails
    update_config(
        &mut app,
        names.clone(),
        Some(other_token.to_string()),
        Some(PaymentInfo::NativePayment {
            token_denom: "ujunox".to_string(),
            payment_amount: Uint128::new(50),
        }),
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap_err();

    // Update config as admin
    update_config(
        &mut app,
        names.clone(),
        Some(NON_ADMIN_ADDR.to_string()),
        Some(PaymentInfo::NativePayment {
            token_denom: "ujunox".to_string(),
            payment_amount: Uint128::new(25),
        }),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(NON_ADMIN_ADDR),
            payment_info: PaymentInfo::NativePayment {
                token_denom: "ujunox".to_string(),
                payment_amount: Uint128::new(25)
            }
        }
    );

    // Update one config value but not the others

    // Only admin
    update_config(
        &mut app,
        names.clone(),
        Some(ADMIN_ADDR.to_string()),
        None,
        Addr::unchecked(NON_ADMIN_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR), // Only this has changed
            payment_info: PaymentInfo::NativePayment {
                token_denom: "ujunox".to_string(),
                payment_amount: Uint128::new(25)
            }
        }
    );

    // Only payment info
    update_config(
        &mut app,
        names.clone(),
        None,
        Some(PaymentInfo::NativePayment {
            token_denom: "uatom".to_string(),
            payment_amount: Uint128::new(50),
        }),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names);
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR),
            payment_info: PaymentInfo::NativePayment {
                token_denom: "uatom".to_string(),
                payment_amount: Uint128::new(50)
            }
        }
    );
}
