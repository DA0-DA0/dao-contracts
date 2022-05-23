use cosmwasm_std::{to_binary, Addr, Binary, Coin, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw_utils::Duration;
use schemars::_private::NoSerialize;

use crate::msg::{ExecuteMsg, InstantiateMsg};

const CREATOR_ADDR: &str = "creator";
const FEE_DENOM: &str = "ujuno";

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
        cw20_staked_balance_voting::contract::execute,
        cw20_staked_balance_voting::contract::instantiate,
        cw20_staked_balance_voting::contract::query,
    )
    .with_reply(cw20_staked_balance_voting::contract::reply);
    Box::new(contract)
}

fn stake_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        stake_cw20::contract::execute,
        stake_cw20::contract::instantiate,
        stake_cw20::contract::query,
    );
    Box::new(contract)
}

fn distribution_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

struct SetupTestResponse {
    app: App,
    dist_addr: Addr,
    staking_addr: Addr,
    token_addr: Addr,
}

fn setup_test(initial_balances: Vec<Cw20Coin>) -> SetupTestResponse {
    let mut app = App::default();
    let voting_id = app.store_code(staked_balances_voting_contract());
    let cw20_id = app.store_code(cw20_contract());
    let dist_id = app.store_code(distribution_contract());
    let stake_cw20_id = app.store_code(stake_cw20());

    let voting_addr = app
        .instantiate_contract(
            voting_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_staked_balance_voting::msg::InstantiateMsg {
                active_threshold: None,
                token_info: cw20_staked_balance_voting::msg::TokenInfo::New {
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
            voting_addr.clone(),
            &cw20_staked_balance_voting::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &cw20_staked_balance_voting::msg::QueryMsg::TokenContract {},
        )
        .unwrap();

    for Cw20Coin { address, amount } in initial_balances {
        app.execute_contract(
            Addr::unchecked(address),
            token_contract.clone(),
            &cw20_base::msg::ExecuteMsg::Send {
                contract: staking_contract.to_string(),
                amount,
                msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
    }

    // Update block so staked balances are reflected.
    app.update_block(next_block);

    let dist_addr = app
        .instantiate_contract(
            dist_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: None,
                voting_contract: voting_addr.to_string(),
                distribution_height: app.block_info().height,
            },
            &[],
            "distribution contract",
            None,
        )
        .unwrap();

    SetupTestResponse {
        app,
        staking_addr: staking_contract,
        dist_addr,
        token_addr: token_contract,
    }
}

fn mint_tokens(app: &mut App, receiver: &str, amount: Uint128, token_contract: Option<Addr>) {
    match token_contract {
        Some(token_contract) => app
            .execute_contract(
                // Creator the minter for the cw20.
                Addr::unchecked(CREATOR_ADDR),
                token_contract,
                &cw20::Cw20ExecuteMsg::Mint {
                    recipient: receiver.to_string(),
                    amount,
                },
                &[],
            )
            .unwrap(),
        None => app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: receiver.to_string(),
                amount: vec![Coin {
                    amount,
                    denom: FEE_DENOM.to_string(),
                }],
            }))
            .unwrap(),
    };
}

fn fund_tokens(
    app: &mut App,
    dist_addr: Addr,
    sender: &str,
    amount: Uint128,
    token_contract: Option<Addr>,
) {
    match token_contract {
        Some(token_contract) => app
            .execute_contract(
                Addr::unchecked(sender),
                token_contract,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: dist_addr.to_string(),
                    amount,
                    msg: Binary::default(),
                },
                &[],
            )
            .unwrap(),
        None => app
            .execute_contract(
                Addr::unchecked(sender),
                dist_addr.clone(),
                &ExecuteMsg::Fund {},
                &[Coin {
                    amount,
                    denom: FEE_DENOM.to_string(),
                }],
            )
            .unwrap(),
    };
}

fn claim_and_assert_tokens(
    app: &mut App,
    sender: &str,
    dist_addr: Addr,
    expected_fee_balance: Uint128,
    token_contract: Option<Addr>,
) {
    let balance = match token_contract {
        Some(token_contract) => {
            app.execute_contract(
                Addr::unchecked(sender),
                dist_addr,
                &ExecuteMsg::ClaimCw20s { tokens: None },
                &[],
            )
            .unwrap();
            let balance: cw20::BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract,
                    &cw20::Cw20QueryMsg::Balance {
                        address: sender.to_string(),
                    },
                )
                .unwrap();
            balance.balance
        }
        None => {
            app.execute_contract(
                Addr::unchecked(sender),
                dist_addr,
                &ExecuteMsg::ClaimNatives { denoms: None },
                &[],
            )
            .unwrap();
            let balance = app
                .wrap()
                .query_balance(sender.to_string(), FEE_DENOM.to_string())
                .unwrap();
            balance.amount
        }
    };
    assert_eq!(balance, expected_fee_balance);
}

fn unstake_tokens(app: &mut App, staking_contract: Addr, sender: &str, amount: Uint128) {
    app.execute_contract(
        Addr::unchecked(sender),
        staking_contract,
        &stake_cw20::msg::ExecuteMsg::Unstake { amount },
        &[],
    )
    .unwrap();
}

fn test_simple(use_cw20s: bool) {
    let SetupTestResponse {
        mut app,
        dist_addr,
        token_addr: token_contract,
        ..
    } = setup_test(vec![Cw20Coin {
        address: "ekez".to_string(),
        amount: Uint128::new(10),
    }]);

    let token_contract = if use_cw20s {
        Some(token_contract)
    } else {
        None
    };

    mint_tokens(
        &mut app,
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );
    fund_tokens(
        &mut app,
        dist_addr.clone(),
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );
    claim_and_assert_tokens(
        &mut app,
        "ekez",
        dist_addr,
        Uint128::new(100),
        token_contract,
    );
}

#[test]
fn test_simple_native() {
    test_simple(false)
}

#[test]
fn test_simple_cw20() {
    test_simple(true)
}

fn test_claim_claim_again(use_cw20s: bool) {
    let SetupTestResponse {
        mut app,
        dist_addr,
        token_addr: token_contract,
        ..
    } = setup_test(vec![
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "floob".to_string(),
            amount: Uint128::new(10),
        },
    ]);

    let token_contract = if use_cw20s {
        Some(token_contract)
    } else {
        None
    };

    mint_tokens(
        &mut app,
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );
    fund_tokens(
        &mut app,
        dist_addr.clone(),
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );

    claim_and_assert_tokens(
        &mut app,
        "ekez",
        dist_addr.clone(),
        Uint128::new(50),
        token_contract.clone(),
    );

    // Do another funding round.
    mint_tokens(
        &mut app,
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );
    fund_tokens(
        &mut app,
        dist_addr.clone(),
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );

    claim_and_assert_tokens(
        &mut app,
        "ekez",
        dist_addr.clone(),
        Uint128::new(100),
        token_contract.clone(),
    );

    claim_and_assert_tokens(
        &mut app,
        "floob",
        dist_addr,
        Uint128::new(100),
        token_contract,
    );
}

#[test]
fn test_claim_claim_again_cw20() {
    test_claim_claim_again(true)
}

#[test]
fn test_claim_claim_again_native() {
    test_claim_claim_again(false)
}

fn test_unstake_post_distribution(use_cw20s: bool) {
    let SetupTestResponse {
        mut app,
        dist_addr,
        token_addr: token_contract,
        staking_addr,
    } = setup_test(vec![
        Cw20Coin {
            address: "ekez".to_string(),
            amount: Uint128::new(10),
        },
        Cw20Coin {
            address: "floob".to_string(),
            amount: Uint128::new(10),
        },
    ]);

    let token_contract = if use_cw20s {
        Some(token_contract)
    } else {
        None
    };

    mint_tokens(
        &mut app,
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );
    fund_tokens(
        &mut app,
        dist_addr.clone(),
        CREATOR_ADDR,
        Uint128::new(100),
        token_contract.clone(),
    );

    // Unstake tokens. This shouldn't matter because distribution
    // ought to happen from the time of contract creation.
    unstake_tokens(&mut app, staking_addr.clone(), "ekez", Uint128::new(10));

    app.update_block(next_block);

    claim_and_assert_tokens(
        &mut app,
        "ekez",
        dist_addr.clone(),
        Uint128::new(if use_cw20s { 60 } else { 50 }),
        token_contract.clone(),
    );

    claim_and_assert_tokens(
        &mut app,
        "floob",
        dist_addr,
        Uint128::new(50),
        token_contract,
    );
}

#[test]
fn test_unstake_post_distribution_cw20() {
    test_unstake_post_distribution(true)
}

#[test]
fn test_unstake_post_distribution_native() {
    test_unstake_post_distribution(false)
}
