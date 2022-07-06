use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_core::{msg::ModuleInstantiateInfo, query::DumpStateResponse};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::Duration;

use voting::threshold::Threshold;

use crate::msg::InstantiateMsg;

const WHALE_ADDR: &str = "whale";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn single_govmod_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

fn staked_balances_voting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_staked_balance_voting::contract::execute,
        cw20_staked_balance_voting::contract::instantiate,
        cw20_staked_balance_voting::contract::query,
    )
    .with_reply(cw20_staked_balance_voting::contract::reply);
    Box::new(contract)
}

fn cw20_stake() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
    );
    Box::new(contract)
}

fn core_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_core::contract::execute,
        cw_core::contract::instantiate,
        cw_core::contract::query,
    )
    .with_reply(cw_core::contract::reply);
    Box::new(contract)
}

#[test]
fn instantiate_with_staked_balances_voting() {
    let mut app = App::default();

    let govmod_id = app.store_code(single_govmod_contract());
    let cw20_id = app.store_code(cw20_contract());
    let cw20_stake_id = app.store_code(cw20_stake());
    let core_contract_id = app.store_code(core_contract());
    let staked_balances_voting_id = app.store_code(staked_balances_voting());

    let instantiate_core = cw_core::msg::InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: staked_balances_voting_id,
            msg: to_binary(&cw20_staked_balance_voting::msg::InstantiateMsg {
                active_threshold: None,
                token_info: cw20_staked_balance_voting::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: WHALE_ADDR.to_string(),
                        amount: Uint128::new(100),
                    }],
                    marketing: None,
                    staking_code_id: cw20_stake_id,
                    unstaking_duration: Some(Duration::Height(10u64)),
                    initial_dao_balance: Some(Uint128::new(100)),
                },
            })
            .unwrap(),
            admin: cw_core::msg::Admin::None {},
            label: "DAO DAO voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            label: "DAO DAO governance module.".to_string(),
            admin: cw_core::msg::Admin::CoreContract {},
            msg: to_binary(&InstantiateMsg {
                threshold: Threshold::ThresholdQuorum {
                    threshold: voting::threshold::PercentageThreshold::Majority {},
                    quorum: voting::threshold::PercentageThreshold::Percent(Decimal::percent(30)),
                },
                max_voting_period: Duration::Height(10u64),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                deposit_info: None,
            })
            .unwrap(),
        }],
        initial_items: None,
    };

    let core_addr = app
        .instantiate_contract(
            core_contract_id,
            Addr::unchecked(WHALE_ADDR),
            &instantiate_core,
            &[],
            "DAO DAO",
            None,
        )
        .unwrap();

    let state: DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();

    assert_eq!(state.proposal_modules.len(), 1);
    assert_eq!(
        state.config,
        cw_core::state::Config {
            name: "DAO DAO".to_string(),
            description: "A DAO that builds DAOs".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: false,
        }
    );
}
