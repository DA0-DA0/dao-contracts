use anyhow::{Context, Result};
use cosm_orc::{
    config::cfg::Config,
    orchestrator::cosm_orc::{CosmOrc, WasmMsg},
    profilers::gas_profiler::GasProfiler,
};
use cosmwasm_std::{to_binary, CosmosMsg, Decimal, Uint128};
use cw20::Cw20Coin;
use cw_utils::Duration;
use serde_json::Value;
use std::env;
use std::fs;
use voting::{
    deposit::DepositInfo, deposit::DepositToken, threshold::PercentageThreshold,
    threshold::Threshold,
};

// TODO: Make integration tests using cosm-orc in a similar fashion to these gas benchmarks

// TODO: Can I use codecov to show the coverage of just the gas profiler separate from the unit tests?

fn main() -> Result<()> {
    let gas_report_out = env::var("GAS_REPORT_OUT")?;
    let admin_addr = env::var("ADMIN_ADDR")?;
    let contract_dir = env::var("CONTRACT_DIR")?;

    env_logger::init();

    let mut cosm_orc =
        CosmOrc::new(Config::from_yaml("config.yaml")?).add_profiler(Box::new(GasProfiler::new()));

    cosm_orc.store_contracts(&contract_dir)?;

    // ### CW-CORE ###
    cw_core_admin_benchmark(&mut cosm_orc, admin_addr)?;
    cw_core_item_benchmark(&mut cosm_orc)?;
    cw_core_pause_benchmark(&mut cosm_orc)?;

    // TODO:
    // * Do more cw-core benchmarks
    // * Add benchmarks for the rests of the contracts

    // Write output file:
    let reports = cosm_orc.profiler_reports()?;

    let j: Value = serde_json::from_slice(&reports[0].json_data)?;
    fs::write(gas_report_out, j.to_string())?;

    Ok(())
}

fn cw_core_admin_benchmark(cosm_orc: &mut CosmOrc, admin_addr: String) -> Result<()> {
    let msgs: Vec<
        WasmMsg<cw_core::msg::InstantiateMsg, cw_core::msg::ExecuteMsg, cw_core::msg::QueryMsg>,
    > = vec![
        WasmMsg::InstantiateMsg(cw_core::msg::InstantiateMsg {
            admin: Some(admin_addr.clone()),
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false,
            voting_module_instantiate_info: cw_core::msg::ModuleInstantiateInfo {
                code_id: cosm_orc
                    .contract_map
                    .get("cw20_staked_balance_voting")
                    .context("not deployed")?
                    .code_id,
                msg: to_binary(&cw20_staked_balance_voting::msg::InstantiateMsg {
                    token_info: cw20_staked_balance_voting::msg::TokenInfo::New {
                        code_id: cosm_orc
                            .contract_map
                            .get("cw20_base")
                            .context("not deployed")?
                            .code_id,
                        label: "DAO DAO Gov token".to_string(),
                        name: "DAO".to_string(),
                        symbol: "DAO".to_string(),
                        decimals: 6,
                        initial_balances: vec![Cw20Coin {
                            address: "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string(),
                            amount: Uint128::new(1000000000000000),
                        }],
                        marketing: None,
                        staking_code_id: cosm_orc
                            .contract_map
                            .get("cw20_stake")
                            .context("not deployed")?
                            .code_id,
                        unstaking_duration: Some(Duration::Time(1209600)),
                        initial_dao_balance: None,
                    },
                    active_threshold: None,
                })?,
                admin: cw_core::msg::Admin::CoreContract {},
                label: "DAO DAO Voting Module".to_string(),
            },
            proposal_modules_instantiate_info: vec![cw_core::msg::ModuleInstantiateInfo {
                code_id: cosm_orc
                    .contract_map
                    .get("cw_proposal_single")
                    .context("not deployed")?
                    .code_id,
                msg: to_binary(&cw_proposal_single::msg::InstantiateMsg {
                    min_voting_period: None,
                    threshold: Threshold::ThresholdQuorum {
                        threshold: PercentageThreshold::Majority {},
                        quorum: PercentageThreshold::Percent(Decimal::percent(1)),
                    },
                    max_voting_period: Duration::Time(432000),
                    allow_revoting: false,
                    only_members_execute: true,
                    deposit_info: Some(DepositInfo {
                        token: DepositToken::VotingModuleToken {},
                        deposit: Uint128::new(1000000000),
                        refund_failed_proposals: true,
                    }),
                })?,
                admin: cw_core::msg::Admin::CoreContract {},
                label: "DAO DAO Proposal Module".to_string(),
            }],
            initial_items: None,
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::DumpState {}),
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::NominateAdmin {
            admin: Some(admin_addr.clone()),
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::AdminNomination {}),
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::AcceptAdminNomination {}),
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::NominateAdmin {
            admin: Some(admin_addr),
        }),
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::WithdrawAdminNomination {}),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::AdminNomination {}),
    ];

    cosm_orc.process_msgs("cw_core".to_string(), &msgs)?;

    Ok(())
}

fn cw_core_item_benchmark(cosm_orc: &mut CosmOrc) -> Result<()> {
    // Uses `cw_core` deployed contract address, to avoid re-initializing it on chain.
    // If you wish to make a new `cw_core`, simply pass in an `InstantiateMsg` again.
    let msgs: Vec<
        WasmMsg<cw_core::msg::InstantiateMsg, cw_core::msg::ExecuteMsg, cw_core::msg::QueryMsg>,
    > = vec![
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::DumpState {}),
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: cosm_orc
                    .contract_map
                    .get("cw_core")
                    .context("not stored")?
                    .address
                    .as_ref()
                    .context("not deployed")?
                    .to_string(),
                msg: to_binary(&cw_core::msg::ExecuteMsg::SetItem {
                    key: "meme".to_string(),
                    addr: "junomeme".to_string(),
                })
                .unwrap(),
                funds: vec![],
            })],
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
            key: "meme".to_string(),
        }),
    ];

    cosm_orc.process_msgs("cw_core".to_string(), &msgs)?;

    Ok(())
}

fn cw_core_pause_benchmark(cosm_orc: &mut CosmOrc) -> Result<()> {
    let msgs: Vec<
        WasmMsg<cw_core::msg::InstantiateMsg, cw_core::msg::ExecuteMsg, cw_core::msg::QueryMsg>,
    > = vec![
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: cosm_orc
                    .contract_map
                    .get("cw_core")
                    .context("not stored")?
                    .address
                    .as_ref()
                    .context("not deployed")?
                    .to_string(),
                msg: to_binary(&cw_core::msg::ExecuteMsg::Pause {
                    duration: Duration::Time(1),
                })
                .unwrap(),
                funds: vec![],
            })],
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::DumpState {}),
    ];

    cosm_orc.process_msgs("cw_core".to_string(), &msgs)?;

    Ok(())
}
