use anyhow::Result;
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
use std::{thread, time};
use voting::{
    deposit::DepositInfo, deposit::DepositToken, threshold::PercentageThreshold,
    threshold::Threshold,
};

fn main() -> Result<()> {
    let gas_report_out = env::var("GAS_REPORT_OUT")?;
    let admin_addr = env::var("ADMIN_ADDR")?;
    let contract_dir = env::var("CONTRACT_DIR")?;

    env_logger::init();

    let mut cosm_orc =
        CosmOrc::new(Config::from_yaml("config.yaml")?).add_profiler(Box::new(GasProfiler::new()));

    cosm_orc.store_contracts(&contract_dir)?;

    // ### CW-CORE ###
    cw_core_admin_benchmark(&mut cosm_orc, admin_addr.clone())?;
    cw_core_item_benchmark(&mut cosm_orc)?;
    cw_core_pause_benchmark(&mut cosm_orc)?;

    // ### CW20-STAKED-BALANCE-VOTING ###
    cw20_stake_tokens_benchmark(&mut cosm_orc, admin_addr)?;

    // Write output file:
    let reports = cosm_orc.profiler_reports()?;

    let j: Value = serde_json::from_slice(&reports[0].json_data)?;
    fs::write(gas_report_out, j.to_string())?;

    Ok(())
}

fn cw_core_admin_benchmark(cosm_orc: &mut CosmOrc, admin_addr: String) -> Result<()> {
    let msgs: Vec<CoreWasmMsg> = vec![
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
                    .unwrap()
                    .code_id,
                msg: to_binary(&cw20_staked_balance_voting::msg::InstantiateMsg {
                    token_info: cw20_staked_balance_voting::msg::TokenInfo::New {
                        code_id: cosm_orc.contract_map.get("cw20_base").unwrap().code_id,
                        label: "DAO DAO Gov token".to_string(),
                        name: "DAO".to_string(),
                        symbol: "DAO".to_string(),
                        decimals: 6,
                        initial_balances: vec![Cw20Coin {
                            address: admin_addr.clone(),
                            amount: Uint128::new(1000000000000000),
                        }],
                        marketing: None,
                        staking_code_id: cosm_orc.contract_map.get("cw20_stake").unwrap().code_id,
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
                    .unwrap()
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
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::VotingModule {}),
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
    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::DumpState {}),
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: cosm_orc
                    .contract_map
                    .get("cw_core")
                    .unwrap()
                    .address
                    .as_ref()
                    .unwrap()
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
    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: cosm_orc
                    .contract_map
                    .get("cw_core")
                    .unwrap()
                    .address
                    .as_ref()
                    .unwrap()
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

fn cw20_stake_tokens_benchmark(cosm_orc: &mut CosmOrc, admin_addr: String) -> Result<()> {
    // Store contract addresses for new dao's staking and voting modules:

    // TODO: Just parse these from the cw_core instantiate respone json instead
    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::VotingModule {});
    let res = cosm_orc.process_msg("cw_core".to_string(), &msg)?;
    let voting_module_addr = &res["data"].as_str().unwrap();

    // TODO: make this all nicer to use in an integration test harness lib
    cosm_orc
        .contract_map
        .get_mut(&"cw20_staked_balance_voting".to_string())
        .unwrap()
        .address = Some(voting_module_addr.to_string());

    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::StakingContract {});
    let res = cosm_orc.process_msg("cw20_staked_balance_voting".to_string(), &msg)?;
    let staking_addr = &res["data"].as_str().unwrap();

    cosm_orc
        .contract_map
        .get_mut(&"cw20_stake".to_string())
        .unwrap()
        .address = Some(staking_addr.to_string());

    // Stake tokens:
    let msgs: Vec<Cw20StakeWasmMsg> = vec![
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
            address: admin_addr.clone(),
        }),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::GetConfig {}),
    ];
    let res = cosm_orc.process_msgs("cw20_stake".to_string(), &msgs)?;
    let token_addr = &res[1]["data"]["token_address"].as_str().unwrap();

    cosm_orc
        .contract_map
        .get_mut(&"cw20_base".to_string())
        .unwrap()
        .address = Some(token_addr.to_string());

    let msgs: Vec<Cw20BaseWasmMsg> = vec![WasmMsg::ExecuteMsg(cw20_base::msg::ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    })];
    cosm_orc.process_msgs("cw20_base".to_string(), &msgs)?;

    let msgs: Vec<Cw20StakeWasmMsg> =
        vec![WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
            address: admin_addr,
        })];
    cosm_orc.process_msgs("cw20_stake".to_string(), &msgs)?;

    // Sleep to let staking block process:
    thread::sleep(time::Duration::from_millis(5000));

    // TODO: Unstake a token

    Ok(())
}

// TODO: Use a macro for these type aliases? (put it in cosm-orc actually)
type CoreWasmMsg =
    WasmMsg<cw_core::msg::InstantiateMsg, cw_core::msg::ExecuteMsg, cw_core::msg::QueryMsg>;

type Cw20StakeBalanceWasmMsg = WasmMsg<
    cw20_staked_balance_voting::msg::InstantiateMsg,
    cw20_staked_balance_voting::msg::ExecuteMsg,
    cw20_staked_balance_voting::msg::QueryMsg,
>;

type Cw20StakeWasmMsg = WasmMsg<
    cw20_stake::msg::InstantiateMsg,
    cw20_stake::msg::ExecuteMsg,
    cw20_stake::msg::QueryMsg,
>;

type Cw20BaseWasmMsg =
    WasmMsg<cw20_base::msg::InstantiateMsg, cw20_base::msg::ExecuteMsg, cw20_base::msg::QueryMsg>;
