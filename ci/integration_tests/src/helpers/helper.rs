use crate::test_harness::chain::Chain;
use anyhow::Result;
use cosm_orc::orchestrator::cosm_orc::WasmMsg;
use cosmwasm_std::{to_binary, Decimal, Uint128};
use cw20::Cw20Coin;
use cw_core::{
    msg::{Admin, ModuleInstantiateInfo},
    query::DumpStateResponse,
};
use cw_utils::Duration;
use voting::{
    deposit::DepositInfo, deposit::DepositToken, threshold::PercentageThreshold,
    threshold::Threshold,
};

pub struct DaoState {
    pub addr: String,
    pub state: DumpStateResponse,
}

pub fn create_dao(
    admin: Option<String>,
    user_addr: String,
    voting_contract: &str,
    proposal_contract: &str,
) -> Result<DaoState> {
    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::InstantiateMsg(cw_core::msg::InstantiateMsg {
            admin,
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: Chain::contract_code_id(voting_contract),
                msg: to_binary(&cw20_staked_balance_voting::msg::InstantiateMsg {
                    token_info: cw20_staked_balance_voting::msg::TokenInfo::New {
                        code_id: Chain::contract_code_id("cw20_base"),
                        label: "DAO DAO Gov token".to_string(),
                        name: "DAO".to_string(),
                        symbol: "DAO".to_string(),
                        decimals: 6,
                        initial_balances: vec![Cw20Coin {
                            address: user_addr,
                            amount: Uint128::new(100_000_000),
                        }],
                        marketing: None,
                        staking_code_id: Chain::contract_code_id("cw20_stake"),
                        unstaking_duration: Some(Duration::Time(1209600)),
                        initial_dao_balance: None,
                    },
                    active_threshold: None,
                })?,
                admin: Admin::CoreContract {},
                label: "DAO DAO Voting Module".to_string(),
            },
            proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                code_id: Chain::contract_code_id(proposal_contract),
                msg: to_binary(&cw_proposal_single::msg::InstantiateMsg {
                    min_voting_period: None,
                    threshold: Threshold::ThresholdQuorum {
                        threshold: PercentageThreshold::Majority {},
                        quorum: PercentageThreshold::Percent(Decimal::percent(35)),
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
                admin: Admin::CoreContract {},
                label: "DAO DAO Proposal Module".to_string(),
            }],
            initial_items: None,
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::DumpState {}),
    ];
    let res = Chain::process_msgs("cw_core".to_string(), &msgs)?;
    let state: DumpStateResponse = serde_json::from_value(res[1]["data"].clone())?;

    Ok(DaoState {
        addr: Chain::contract_addr("cw_core"),
        state,
    })
}

// TODO: Use a macro for these type aliases? (put this in cosm-orc)

pub type CoreWasmMsg =
    WasmMsg<cw_core::msg::InstantiateMsg, cw_core::msg::ExecuteMsg, cw_core::msg::QueryMsg>;

pub type Cw20StakeBalanceWasmMsg = WasmMsg<
    cw20_staked_balance_voting::msg::InstantiateMsg,
    cw20_staked_balance_voting::msg::ExecuteMsg,
    cw20_staked_balance_voting::msg::QueryMsg,
>;

pub type Cw20StakeWasmMsg = WasmMsg<
    cw20_stake::msg::InstantiateMsg,
    cw20_stake::msg::ExecuteMsg,
    cw20_stake::msg::QueryMsg,
>;

pub type Cw20BaseWasmMsg =
    WasmMsg<cw20_base::msg::InstantiateMsg, cw20_base::msg::ExecuteMsg, cw20_base::msg::QueryMsg>;

pub type CwProposalWasmMsg = WasmMsg<
    cw_proposal_single::msg::InstantiateMsg,
    cw_proposal_single::msg::ExecuteMsg,
    cw_proposal_single::msg::QueryMsg,
>;
