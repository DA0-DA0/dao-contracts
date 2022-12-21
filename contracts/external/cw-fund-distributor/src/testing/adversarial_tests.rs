use std::ops::Add;
use cosmwasm_std::{Addr, Binary, Coin, Empty, to_binary, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, next_block, SudoMsg};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::msg::ExecuteMsg::ClaimAll;
use crate::testing::tests::{mint_natives};

const CREATOR_ADDR: &str = "creator";
const FEE_DENOM: &str = "ujuno";

struct BaseTest {
    app: App,
    distributor_address: Addr,
    staking_address: Addr,
    token_address: Addr,
}

fn distributor_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
        .with_migrate(crate::contract::migrate);
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
            Some(CREATOR_ADDR.parse().unwrap()),
        )
        .unwrap();

    BaseTest {
        app,
        distributor_address: distribution_contract,
        staking_address: staking_contract,
        token_address: token_contract,
    }
}

#[test]
pub fn test_claim_lots_of_tokens() {
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

    let token_count = 5000;
    // mint and fund the distributor contract with
    // a bunch of tokens
    for n in 1..token_count {
        let denom = FEE_DENOM.to_owned() + &n.to_string();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: CREATOR_ADDR.to_string(),
            amount: vec![Coin {
                amount,
                denom: denom.clone(),
            }],
        }))
        .unwrap();

        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            distributor_address.clone(),
            &ExecuteMsg::FundNative {},
            &[Coin {
                amount,
                denom: denom.clone(),
            }],
        )
        .unwrap();

        println!("minted & funded {:?}", denom.clone());
    }

    app.execute_contract(
        Addr::unchecked("bekauz"),
        distributor_address.clone(),
        &ClaimAll {},
        &[],
    )
    .unwrap();

    for n in 1..token_count {
        let denom = FEE_DENOM.to_owned() + &n.to_string();

        let expected_balance = amount
            .checked_multiply_ratio(
                Uint128::new(10),
                Uint128::new(30)
            ).unwrap();

        let user_balance_after_claim = app
            .wrap()
            .query_balance("bekauz".to_string(), denom)
            .unwrap();
        assert_eq!(expected_balance, user_balance_after_claim.amount);
    }
}