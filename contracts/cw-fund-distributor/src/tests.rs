use cosmwasm_std::{Addr, Empty};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::Duration;

use crate::msg::InstantiateMsg;

const CREATOR_ADDR: &str = "creator";

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
    voting_addr: Addr,
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
                    initial_balances,
                    marketing: None,
                    staking_code_id: stake_cw20_id,
                    unstaking_duration: Some(Duration::Height(10u64)),
                    initial_dao_balance: None,
                },
            },
            &[],
            "voting contract",
            None,
        )
        .unwrap();

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
        voting_addr,
        dist_addr,
    }
}
