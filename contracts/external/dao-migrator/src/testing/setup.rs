use cosmwasm_std::{to_binary, Addr, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, Executor};

use super::helpers::{InitDaoDataV1, SENDER_ADDR};

/// Instantiate a basic DAO with proposal and voting modules.
pub fn init_dao_v1(mut app: App, data: Option<InitDaoDataV1>) -> (Addr, Addr, Addr, Addr, Addr) {
    let data = data.unwrap_or(InitDaoDataV1::default());
    let sender = Addr::unchecked(SENDER_ADDR);

    // Store codes
    let proposal_code = app.store_code(data.proposal_code);
    let core_code = app.store_code(data.core_code);
    let cw20_code = app.store_code(data.cw20_code);
    let cw20_stake_code = app.store_code(data.cw20_stake_code);
    let voting_code = app.store_code(data.voting_code);

    let initial_balances = vec![Cw20Coin {
        address: SENDER_ADDR.to_string(),
        amount: Uint128::new(2),
    }];

    let core_addr = app
        .instantiate_contract(
            core_code,
            sender.clone(),
            &cw_core_v1::msg::InstantiateMsg {
                admin: Some(SENDER_ADDR.to_string()),
                name: "n".to_string(),
                description: "d".to_string(),
                image_url: Some("i".to_string()),
                automatically_add_cw20s: false,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: cw_core_v1::msg::ModuleInstantiateInfo {
                    code_id: voting_code,
                    msg: to_binary(&dao_voting_cw20_staked::msg::InstantiateMsg {
                        active_threshold: None,
                        token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                            code_id: cw20_code,
                            label: "token".to_string(),
                            name: "name".to_string(),
                            symbol: "symbol".to_string(),
                            decimals: 6,
                            initial_balances,
                            marketing: None,
                            staking_code_id: cw20_stake_code,
                            unstaking_duration: None,
                            initial_dao_balance: Some(Uint128::new(100)),
                        },
                    })
                    .unwrap(),
                    admin: cw_core_v1::msg::Admin::CoreContract {},
                    label: "voting".to_string(),
                },
                proposal_modules_instantiate_info: vec![cw_core_v1::msg::ModuleInstantiateInfo {
                    code_id: proposal_code,
                    msg: to_binary(&cw_proposal_single_v1::msg::InstantiateMsg {
                        threshold: voting_v1::Threshold::AbsolutePercentage {
                            percentage: voting_v1::PercentageThreshold::Majority {},
                        },
                        max_voting_period: cw_utils_v1::Duration::Height(6),
                        min_voting_period: None,
                        only_members_execute: false,
                        allow_revoting: false,
                        deposit_info: None,
                    })
                    .unwrap(),
                    admin: cw_core_v1::msg::Admin::CoreContract {},
                    label: "proposal".to_string(),
                }],
                initial_items: Some(vec![cw_core_v1::msg::InitialItem {
                    key: "key".to_string(),
                    value: "value".to_string(),
                }]),
            },
            &[],
            "core",
            Some(sender.to_string()),
        )
        .unwrap();

    app.execute(
        sender.clone(),
        WasmMsg::UpdateAdmin {
            contract_addr: core_addr.to_string(),
            admin: core_addr.to_string(),
        }
        .into(),
    )
    .unwrap();

    // Get modules addrs
    let proposal_addr = {
        let modules: Vec<Addr> = app
            .wrap()
            .query_wasm_smart(
                &core_addr,
                &cw_core_v1::msg::QueryMsg::ProposalModules {
                    start_at: None,
                    limit: None,
                },
            )
            .unwrap();
        assert!(modules.len() == 1);
        modules.into_iter().next().unwrap()
    };
    let voting_addr: Addr = app
        .wrap()
        .query_wasm_smart(&core_addr, &cw_core_v1::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            &voting_addr,
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            &voting_addr,
            &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();

    // Stake token
    app.execute_contract(
        sender.clone(),
        token_addr.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.clone().into_string(),
            amount: Uint128::new(1),
            msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
        },
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // ----
    // create a proposal and add tokens to the treasury.
    // ----

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Propose {
            title: "t".to_string(),
            description: "d".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_addr.to_string(),
                msg: to_binary(&cw_core_v1::msg::ExecuteMsg::UpdateCw20List {
                    to_add: vec![token_addr.to_string()],
                    to_remove: vec![],
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: voting_v1::Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        sender.clone(),
        proposal_addr.clone(),
        &cw_proposal_single_v1::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let tokens: Vec<cw_core_v1::query::Cw20BalanceResponse> = app
        .wrap()
        .query_wasm_smart(
            &core_addr,
            &cw_core_v1::msg::QueryMsg::Cw20Balances {
                start_at: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        tokens,
        vec![cw_core_v1::query::Cw20BalanceResponse {
            addr: token_addr.clone(),
            balance: Uint128::new(100),
        }]
    );

    (
        core_addr,
        proposal_addr,
        voting_addr,
        token_addr,
        staking_addr,
    )
}

#[test]
fn test_migration_v1_v2() {
    let app = App::default();

    // ----
    // instantiate a v1 DAO
    // ----
    let (core_addr, proposal_addr, voting_addr, token_addr, staking_addr) = init_dao_v1(app, None);
}
