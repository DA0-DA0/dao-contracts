use cosmwasm_std::{Addr, Binary, Coin, Empty, to_binary, Uint128};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, next_block, SudoMsg};
use cw20::Cw20Coin;
use crate::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, TotalPowerResponse};

use cosmwasm_std::StdError::GenericErr;
use crate::msg::ExecuteMsg::{ClaimCW20, ClaimNatives};
use crate::msg::QueryMsg::{TotalPower};

const CREATOR_ADDR: &str = "creator";
const FEE_DENOM: &str = "ujuno";

fn distributor_contract() -> Box<dyn Contract<Empty>> {
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

fn staked_balances_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_voting_cw20_staked::contract::execute,
        dao_voting_cw20_staked::contract::instantiate,
        dao_voting_cw20_staked::contract::query,
    )
        .with_reply(dao_voting_cw20_staked::contract::reply);
    Box::new(contract)
}

fn cw20_staking_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
    );
    Box::new(contract)
}

struct BaseTest {
    app: App,
    distributor_address: Addr,
    staking_address: Addr,
    token_address: Addr,
}

fn setup_test(initial_balances: Vec<Cw20Coin>) -> BaseTest {
    let mut app = App::default();
    let distributor_id = app.store_code(distributor_contract());
    let cw20_id = app.store_code(cw20_contract());
    let voting_id = app.store_code(staked_balances_voting_contract());
    let stake_cw20_id = app.store_code(cw20_staking_contract());

    let voting_address = app
        .instantiate_contract(
            voting_id,
            Addr::unchecked(CREATOR_ADDR),
            &dao_voting_cw20_staked::msg::InstantiateMsg {
                active_threshold: None,
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: initial_balances.clone(),
                    marketing: None,
                    staking_code_id: stake_cw20_id,
                    unstaking_duration: None,
                    initial_dao_balance: None,
                },
            },
            &[],
            "voting contract",
            None,
        )
        .unwrap();

    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_address.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_address.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();

    for Cw20Coin { address, amount } in initial_balances {
        app.execute_contract(
            Addr::unchecked(address),
            token_contract.clone(),
            &cw20_base::msg::ExecuteMsg::Send {
                contract: staking_contract.to_string(),
                amount,
                msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
            .unwrap();
    }

    app.update_block(next_block);

    let distribution_contract = app
        .instantiate_contract(
            distributor_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                voting_contract: voting_address.to_string(),
            },
            &[],
            "distribution contract",
            None,
        )
        .unwrap();

    BaseTest {
        app,
        distributor_address: distribution_contract,
        staking_address: staking_contract,
        token_address: token_contract,
    }
}

pub fn query_cw20_balance(
    app: &mut App,
    token_address: Addr,
    account: Addr,
) -> cw20::BalanceResponse {
    app
        .wrap()
        .query_wasm_smart(
            token_address,
            &cw20::Cw20QueryMsg::Balance {
                address: account.into_string(),
            },
        )
        .unwrap()
}

pub fn query_native_balance(
    app: &mut App,
    account: Addr,
) -> Coin {
    app
        .wrap()
        .query_balance(account.to_string(), FEE_DENOM.to_string())
        .unwrap()
}

pub fn mint_cw20s(
    app: &mut App,
    recipient: Addr,
    token_address: Addr,
    amount: Uint128,
    sender: Addr
) {
    app.execute_contract(
        sender,
        token_address,
        &cw20::Cw20ExecuteMsg::Mint {
            recipient: recipient.to_string(),
            amount,
        },
        &[],
    )
        .unwrap();
}

pub fn mint_natives(
    app: &mut App,
    recipient: Addr,
    amount: Uint128,
) {
    app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: recipient.to_string(),
            amount: vec![Coin {
                amount,
                denom: FEE_DENOM.to_string(),
            }],
        }))
        .unwrap();
}

pub fn fund_distributor_contract_cw20(
    app: &mut App,
    distributor_address: Addr,
    token_address: Addr,
    amount: Uint128,
    sender: Addr
) {
    app.execute_contract(
        sender,
        token_address.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: distributor_address.to_string(),
            amount,
            msg: Binary::default(),
        },
        &[],
    )
        .unwrap();
}

pub fn fund_distributor_contract_natives(
    app: &mut App,
    distributor_address: Addr,
    amount: Uint128,
    sender: Addr
) {
    app.execute_contract(
        Addr::unchecked(sender),
        distributor_address,
        &ExecuteMsg::FundNative {},
        &[Coin {
            amount,
            denom: FEE_DENOM.to_string(),
        }],
    )
        .unwrap();
}

#[test]
fn test_instantiate_fails_given_invalid_voting_contract_address() {

    let mut app = App::default();
    let distributor_id = app.store_code(distributor_contract());
    let cw20_id = app.store_code(cw20_contract());
    let voting_id = app.store_code(staked_balances_voting_contract());
    let stake_cw20_id = app.store_code(cw20_staking_contract());

    let initial_balances = vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        }
    ];

    let voting_address = app
        .instantiate_contract(
            voting_id,
            Addr::unchecked(CREATOR_ADDR),
            &dao_voting_cw20_staked::msg::InstantiateMsg {
                active_threshold: None,
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: initial_balances.clone(),
                    marketing: None,
                    staking_code_id: stake_cw20_id,
                    unstaking_duration: None,
                    initial_dao_balance: None,
                },
            },
            &[],
            "voting contract",
            None,
        )
        .unwrap();

    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_address.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_address.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();

    for Cw20Coin { address, amount } in initial_balances {
        app.execute_contract(
            Addr::unchecked(address),
            token_contract.clone(),
            &cw20_base::msg::ExecuteMsg::Send {
                contract: staking_contract.to_string(),
                amount,
                msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
    }

    app.update_block(next_block);

    let expected_error: ContractError = app
        .instantiate_contract(
            distributor_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                voting_contract: "invalid address".to_string(),
            },
            &[],
            "distribution contract",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(expected_error, ContractError::Std(GenericErr { .. })));
}

#[test]
fn test_instantiate_fails_zero_voting_power() {

    let mut app = App::default();
    let distributor_id = app.store_code(distributor_contract());
    let cw20_id = app.store_code(cw20_contract());
    let voting_id = app.store_code(staked_balances_voting_contract());
    let stake_cw20_id = app.store_code(cw20_staking_contract());

    let initial_balances = vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        }
    ];

    let voting_address = app
        .instantiate_contract(
            voting_id,
            Addr::unchecked(CREATOR_ADDR),
            &dao_voting_cw20_staked::msg::InstantiateMsg {
                active_threshold: None,
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: initial_balances.clone(),
                    marketing: None,
                    staking_code_id: stake_cw20_id,
                    unstaking_duration: None,
                    initial_dao_balance: None,
                },
            },
            &[],
            "voting contract",
            None,
        )
        .unwrap();

    app.update_block(next_block);

    let expected_error: ContractError = app
        .instantiate_contract(
            distributor_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                voting_contract: voting_address.to_string(),
            },
            &[],
            "distribution contract",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(expected_error, ContractError::ZeroVotingPower {}));
}

#[test]
fn test_instantiate_cw_fund_distributor() {
    let BaseTest {
        app,
        distributor_address,
        ..
    } = setup_test(vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(20),
        }
    ]);

    let total_power: TotalPowerResponse = app
        .wrap()
        .query_wasm_smart(
            distributor_address.clone(),
            &TotalPower {}
        )
        .unwrap();

    // assert total power has been set correctly
    assert_eq!(total_power.total_power, Uint128::new(30));
}

#[test]
fn test_fund_cw20() {
    let BaseTest {
        mut app,
        distributor_address,
        staking_address: _,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(20),
        }
    ]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        Addr::unchecked(CREATOR_ADDR),
        token_address.clone(),
        amount,
        Addr::unchecked(CREATOR_ADDR),
    );

    // fund the contract
    fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        Addr::unchecked(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(
        &mut app,
        token_address,
        distributor_address
    );

    assert_eq!(balance.balance, amount);
}

#[test]
pub fn test_fund_natives() {
    let BaseTest {
        mut app,
        distributor_address,
        staking_address: _,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(20),
        }
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, Addr::unchecked(CREATOR_ADDR), amount);

    fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        Addr::unchecked(CREATOR_ADDR)
    );

    let balance = query_native_balance(&mut app, distributor_address).amount;

    assert_eq!(amount, balance);
}

#[test]
pub fn test_claim_cw20() {
    let BaseTest {
        mut app,
        distributor_address,
        staking_address: _,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(20),
        }
    ]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        Addr::unchecked(CREATOR_ADDR),
        token_address.clone(),
        amount,
        Addr::unchecked(CREATOR_ADDR),
    );

    // fund the contract
    fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        Addr::unchecked(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(
        &mut app,
        token_address.clone(),
        distributor_address.clone(),
    );

    assert_eq!(balance.balance, amount);

    // claim the tokens
    // should result in an entitlement of (10/(10 + 20))%
    // of funds in the distributor contract (166666.666667 floored)
    app.execute_contract(
        Addr::unchecked("bekauz"),
        distributor_address.clone(),
        &ClaimCW20 {
            tokens: Some(vec![token_address.to_string()]),
        },
        &[],
    )
    .unwrap();

    // assert user has received the expected funds
    let expected_balance = amount
        .checked_multiply_ratio(
            Uint128::new(10),
            Uint128::new(30)
        ).unwrap();

    let user_balance_after_claim = query_cw20_balance(
        &mut app,
        token_address.clone(),
        Addr::unchecked("bekauz"),
    );
    assert_eq!(expected_balance, user_balance_after_claim.balance);

    // assert funds have been deducted from distributor
    let distributor_balance_after_claim = query_cw20_balance(
        &mut app,
        token_address.clone(),
        distributor_address.clone(),
    );
    assert_eq!(amount - expected_balance, distributor_balance_after_claim.balance);

    app.update_block(next_block);
    // fund contract again with 10_000
    let new_amount = Uint128::new(100000);
    mint_cw20s(
        &mut app,
        Addr::unchecked(CREATOR_ADDR),
        token_address.clone(),
        new_amount,
        Addr::unchecked(CREATOR_ADDR),
    );
    fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        new_amount,
        Addr::unchecked(CREATOR_ADDR),
    );

    // claim the tokens again
    app.execute_contract(
        Addr::unchecked("bekauz"),
        distributor_address.clone(),
        &ClaimCW20 {
            tokens: Some(vec![token_address.to_string()]),
        },
        &[],
    )
    .unwrap();

    // assert that user has been able to claim the difference
    // between total funded CW20_BALANCES of the token being
    // claimed and the amount previously claimed.
    // while in theory 100000 * (1/3) floored is 33333,
    // 600000 * (1/3) => 200000
    // previous claim => 166666
    // new claim => (200000 - 166666) => 33334
    let total_expected_claim = Uint128::new(200000);

    let user_balance_after_claim = query_cw20_balance(
        &mut app,
        token_address.clone(),
        Addr::unchecked("bekauz"),
    );
    assert_eq!(total_expected_claim, user_balance_after_claim.balance);
}


#[test]
pub fn test_claim_natives() {
    let BaseTest {
        mut app,
        distributor_address,
        staking_address: _,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: "bekauz".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(20),
        }
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, Addr::unchecked(CREATOR_ADDR), amount);

    fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        Addr::unchecked(CREATOR_ADDR)
    );

    app.execute_contract(
        Addr::unchecked("bekauz"),
        distributor_address.clone(),
        &ClaimNatives {
            denoms: Some(vec![FEE_DENOM.to_string()]),
        },
        &[],
    )
    .unwrap();

    let expected_balance = amount
        .checked_multiply_ratio(
            Uint128::new(10),
            Uint128::new(30)
        ).unwrap();

    let user_balance_after_claim = query_native_balance(
        &mut app,
        Addr::unchecked("bekauz"),
    );
    assert_eq!(expected_balance, user_balance_after_claim.amount);

    // assert funds have been deducted from distributor
    let distributor_balance_after_claim = query_native_balance(
        &mut app,
        distributor_address.clone(),
    );
    assert_eq!(amount - expected_balance, distributor_balance_after_claim.amount);
}