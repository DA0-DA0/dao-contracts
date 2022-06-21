use cosmwasm_std::{to_binary, Addr, Coin, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

use crate::{
    msg::{Counterparty, ExecuteMsg, InstantiateMsg, QueryMsg, StatusResponse, TokenInfo},
    state::{CheckedCounterparty, CheckedTokenInfo},
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
        &ExecuteMsg::Fund {},
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
        &ExecuteMsg::Fund {},
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
            },
            counterparty_two: CheckedCounterparty {
                address: Addr::unchecked(DAO2),
                promise: CheckedTokenInfo::Cw20 {
                    contract_addr: cw20.clone(),
                    amount: Uint128::new(100)
                },
                provided: false,
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
            },
            counterparty_two: CheckedCounterparty {
                address: Addr::unchecked(DAO2),
                promise: CheckedTokenInfo::Cw20 {
                    contract_addr: cw20,
                    amount: Uint128::new(100)
                },
                provided: false,
            }
        }
    )
}
