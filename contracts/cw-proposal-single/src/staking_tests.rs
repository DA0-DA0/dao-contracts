use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_core::query::DumpStateResponse;
use cw_core_interface::{Admin, ModuleInstantiateInfo};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::Duration;

use cw_pre_propose_base_proposal_single as cppbps;
use voting::{pre_propose::PreProposeInfo, threshold::Threshold};

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

fn pre_propose_single() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cppbps::contract::execute,
        cppbps::contract::instantiate,
        cppbps::contract::query,
    );
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

    let pre_propose_contract = app.store_code(pre_propose_single());
    let pre_propose_info = PreProposeInfo::ModuleMayPropose {
        info: ModuleInstantiateInfo {
            code_id: pre_propose_contract,
            msg: to_binary(&cppbps::InstantiateMsg {
                deposit_info: None,
                open_proposal_submission: false,
                extension: Empty::default(),
            })
            .unwrap(),
            admin: Some(Admin::Instantiator {}),
            label: "pre_propose_contract".to_string(),
        },
    };

    let instantiate_core = cw_core::msg::InstantiateMsg {
        dao_uri: None,
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
            admin: None,
            label: "DAO DAO voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            label: "DAO DAO governance module.".to_string(),
            admin: Some(Admin::Instantiator {}),
            msg: to_binary(&InstantiateMsg {
                threshold: Threshold::ThresholdQuorum {
                    threshold: voting::threshold::PercentageThreshold::Majority {},
                    quorum: voting::threshold::PercentageThreshold::Percent(Decimal::percent(30)),
                },
                max_voting_period: Duration::Height(10u64),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                pre_propose_info,
                close_proposal_on_execution_failure: true,
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
            dao_uri: None,
            name: "DAO DAO".to_string(),
            description: "A DAO that builds DAOs".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: false,
        }
    );
}
