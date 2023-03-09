use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env},
    to_binary, Addr, Coin, Empty, Uint128,
};
use cw20::Cw20Coin;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{Cw20RecieveMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StatusResponse},
    types::{
        AcceptedMessages, CheckedCounterparty, CheckedTokenInfo, Counterparty, SendMessage,
        TokenInfo,
    },
    ContractError,
};

const DAO1: &str = "dao1";
const DAO2: &str = "dao2";

fn escrow_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
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

fn cw_vesting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_vesting::contract::execute,
        cw_vesting::contract::instantiate,
        cw_vesting::contract::query,
    );
    Box::new(contract)
}

#[test]
fn test_simple_escrow() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint128::new(100),
            msg: to_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow,
        &ExecuteMsg::Fund { send_message: None },
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let dao1_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20,
            &cw20::Cw20QueryMsg::Balance {
                address: DAO1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao1_balance.balance, Uint128::new(100));

    let dao2_balance = app.wrap().query_balance(DAO2, "ujuno").unwrap();
    assert_eq!(dao2_balance.amount, Uint128::new(100))
}

#[test]
fn test_simple_with_send_messages() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());
    let vesting_code = app.store_code(cw_vesting());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let vetsing_init_msg = cw_vesting::msg::InstantiateMsg {
        owner: Some("owner".to_string()),
        recipient: DAO2.to_string(),
        title: "title".to_string(),
        description: Some("description".to_string()),
        total: Uint128::new(200),
        denom: cw_denom::UncheckedDenom::Native("ujuno".to_string()),
        schedule: cw_vesting::vesting::Schedule::SaturatingLinear,
        start_time: None,
        vesting_duration_seconds: 60 * 60 * 24 * 7, // one week
        unbonding_duration_seconds: 60,
    };

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(200),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    let some_other_escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: escrow.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // In this case we are sending cw20 tokens, but expecting to get native token
    // So we can send any set of messages we want here.
    app.execute_contract(
        Addr::unchecked(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint128::new(100),
            msg: to_binary(&Cw20RecieveMsg::FundWithMsgs {
                send_message: SendMessage::SendNative {
                    messages: vec![AcceptedMessages::WasmInstantiate {
                        admin: None,
                        code_id: vesting_code,
                        msg: to_binary(&vetsing_init_msg).unwrap(),
                        funds: coins(200, "ujuno"),
                        label: "some vesting".to_string(),
                    }],
                },
            })
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    // We recieve 100 cw20 token, just for fun, im trying to fund a differetn swap with this swap
    // So once this swap is done, I can fund the other swap with the 50 cw20 tokens
    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow,
        &ExecuteMsg::Fund {
            send_message: Some(SendMessage::SendCw20 {
                contract_address: some_other_escrow.to_string(),
                message: to_binary("").unwrap(),
            }),
        },
        &[Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    // --- Cool everything passed, lets make sure everything is sent correctly ---

    // dao1 cw20 balance should be 0 because we sent it into the other escrow
    let dao1_cw20_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: DAO2.to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao1_cw20_balance.balance, Uint128::new(0));

    // Lets make sure the other escrow was funded correctly
    // provided is true and the cw20 balance is 100
    let other_escrow_status: StatusResponse = app
        .wrap()
        .query_wasm_smart(some_other_escrow.clone(), &QueryMsg::Status {})
        .unwrap();
    let other_escrow_cw20_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20,
            &cw20::Cw20QueryMsg::Balance {
                address: some_other_escrow.to_string(),
            },
        )
        .unwrap();
    assert!(other_escrow_status.counterparty_two.provided);
    assert_eq!(other_escrow_cw20_balance.balance, Uint128::new(100));

    // Make sure that DAO1 native balance is 0 (sent to the vesting contract)
    let dao1_balance = app.wrap().query_balance(DAO1.to_string(), "ujuno").unwrap();
    assert_eq!(dao1_balance.amount, Uint128::new(0));
}

#[test]
fn test_multiple_send_messages() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(200),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    // We recieve 100 cw20 token, just for fun, im trying to fund a differetn swap with this swap
    // So once this swap is done, I can fund the other swap with the 50 cw20 tokens
    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund { send_message: None },
        &[Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO2),
        cw20,
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint128::new(100),
            msg: to_binary(&Cw20RecieveMsg::FundWithMsgs {
                send_message: SendMessage::SendNative {
                    messages: vec![
                        AcceptedMessages::BankBurn {
                            amount: coins(100, "ujuno"),
                        },
                        AcceptedMessages::BankSend {
                            to_address: "some_random".to_string(),
                            amount: coins(100, "ujuno"),
                        },
                    ],
                },
            })
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    let some_random_balance = app
        .wrap()
        .query_balance("some_random".to_string(), "ujuno")
        .unwrap();
    assert_eq!(some_random_balance.amount, Uint128::new(100));
}

#[test]
fn test_send_messages_incomplete_funds() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(200),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    // We recieve 100 cw20 token, just for fun, im trying to fund a differetn swap with this swap
    // So once this swap is done, I can fund the other swap with the 50 cw20 tokens
    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund { send_message: None },
        &[Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let err = app
        .execute_contract(
            Addr::unchecked(DAO2),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint128::new(100),
                msg: to_binary(&Cw20RecieveMsg::FundWithMsgs {
                    send_message: SendMessage::SendNative {
                        messages: vec![AcceptedMessages::BankBurn {
                            amount: coins(100, "ujuno"),
                        }],
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();

    assert_eq!(err, ContractError::WrongFundsCalculation {});
}

#[test]
fn test_withdraw() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // Can't withdraw before you provide.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO2),
            escrow.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoProvision {});

    app.execute_contract(
        Addr::unchecked(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint128::new(100),
            msg: to_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    // Change our minds.
    app.execute_contract(
        Addr::unchecked(DAO2),
        escrow.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();

    let dao2_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: DAO2.to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao2_balance.balance, Uint128::new(100));

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund { send_message: None },
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let status: StatusResponse = app
        .wrap()
        .query_wasm_smart(escrow.clone(), &QueryMsg::Status {})
        .unwrap();
    assert_eq!(
        status,
        StatusResponse {
            counterparty_one: CheckedCounterparty {
                address: Addr::unchecked(DAO1),
                promise: CheckedTokenInfo::Native {
                    denom: "ujuno".to_string(),
                    amount: Uint128::new(100)
                },
                provided: true,
                send_msg: None,
            },
            counterparty_two: CheckedCounterparty {
                address: Addr::unchecked(DAO2),
                promise: CheckedTokenInfo::Cw20 {
                    contract_addr: cw20.clone(),
                    amount: Uint128::new(100)
                },
                provided: false,
                send_msg: None,
            }
        }
    );

    // Change our minds.
    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();

    let dao1_balance = app.wrap().query_balance(DAO1, "ujuno").unwrap();
    assert_eq!(dao1_balance.amount, Uint128::new(100));

    let status: StatusResponse = app
        .wrap()
        .query_wasm_smart(escrow, &QueryMsg::Status {})
        .unwrap();
    assert_eq!(
        status,
        StatusResponse {
            counterparty_one: CheckedCounterparty {
                address: Addr::unchecked(DAO1),
                promise: CheckedTokenInfo::Native {
                    denom: "ujuno".to_string(),
                    amount: Uint128::new(100)
                },
                provided: false,
                send_msg: None,
            },
            counterparty_two: CheckedCounterparty {
                address: Addr::unchecked(DAO2),
                promise: CheckedTokenInfo::Cw20 {
                    contract_addr: cw20,
                    amount: Uint128::new(100)
                },
                provided: false,
                send_msg: None,
            }
        }
    )
}

#[test]
fn test_withdraw_post_completion() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint128::new(100),
            msg: to_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund { send_message: None },
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let dao1_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20,
            &cw20::Cw20QueryMsg::Balance {
                address: DAO1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao1_balance.balance, Uint128::new(100));

    let dao2_balance = app.wrap().query_balance(DAO2, "ujuno").unwrap();
    assert_eq!(dao2_balance.amount, Uint128::new(100));

    let err: ContractError = app
        .execute_contract(Addr::unchecked(DAO1), escrow, &ExecuteMsg::Withdraw {}, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Complete {})
}

#[test]
fn test_invalid_instantiate() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    // Zero amount not allowed for native tokens.
    let err: ContractError = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(0),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::ZeroTokens {}));

    // Zero amount not allowed for cw20 tokens.
    let err: ContractError = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(0),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::ZeroTokens {}))
}

#[test]
fn test_non_distincy_counterparties() {
    let mut app = App::default();

    let escrow_code = app.store_code(escrow_contract());

    // Zero amount not allowed for native tokens.
    let err: ContractError = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(110),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(10),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::NonDistinctCounterparties {}));
}

#[test]
fn test_fund_non_counterparty() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: "noah".to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("noah"),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint128::new(100),
                msg: to_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::Unauthorized {}));

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: "noah".to_string(),
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("noah"),
            escrow,
            &ExecuteMsg::Fund { send_message: None },
            &[Coin {
                amount: Uint128::new(100),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::Unauthorized {}));
}

#[test]
fn test_fund_twice() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(200),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint128::new(100),
            msg: to_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund { send_message: None },
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            escrow.clone(),
            &ExecuteMsg::Fund { send_message: None },
            &[Coin {
                amount: Uint128::new(100),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::AlreadyProvided {}));

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO2),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.into_string(),
                amount: Uint128::new(100),
                msg: to_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::AlreadyProvided {}));
}

#[test]
fn test_fund_invalid_amount() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_contract());
    let escrow_code = app.store_code(escrow_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(200),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO2),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint128::new(10),
                msg: to_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    let expected = ContractError::InvalidAmount {
        expected: Uint128::new(100),
        actual: Uint128::new(10),
    };
    assert_eq!(err, expected);

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            escrow,
            &ExecuteMsg::Fund { send_message: None },
            &[Coin {
                amount: Uint128::new(200),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    let expected = ContractError::InvalidAmount {
        expected: Uint128::new(100),
        actual: Uint128::new(200),
    };
    assert_eq!(err, expected);
}

#[test]
fn test_fund_invalid_denom() {
    let mut app = App::default();

    let escrow_code = app.store_code(escrow_contract());

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Native {
                        denom: "uekez".to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // Coutnerparty one tries to fund in the denom of counterparty
    // two.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: DAO1.to_string(),
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "uekez".to_string(),
        }],
    }))
    .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            escrow,
            &ExecuteMsg::Fund { send_message: None },
            &[Coin {
                amount: Uint128::new(100),
                denom: "uekez".to_string(),
            }],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidFunds {})
}

#[test]
fn test_fund_invalid_cw20() {
    let mut app = App::default();

    let escrow_code = app.store_code(escrow_contract());
    let cw20_code = app.store_code(cw20_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO1.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let bad_cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: DAO2.to_string(),
                    amount: Uint128::new(100),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            Addr::unchecked(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: DAO1.to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(100),
                    },
                },
                counterparty_two: Counterparty {
                    address: DAO2.to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint128::new(100),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // Try and fund the contract with the wrong cw20.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO2),
            bad_cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint128::new(100),
                msg: to_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidFunds {});

    // Try and fund the contract with the correct cw20 but incorrect
    // provider.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint128::new(100),
                msg: to_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidFunds {})
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
