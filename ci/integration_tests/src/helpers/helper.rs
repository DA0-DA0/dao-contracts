use super::chain::Chain;
use anyhow::Result;
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

#[derive(Debug)]
pub struct DaoState {
    pub addr: String,
    pub state: DumpStateResponse,
}

pub fn create_dao(
    chain: &mut Chain,
    admin: Option<String>,
    op_name: &str,
    user_addr: String,
) -> Result<DaoState> {
    let msg = cw_core::msg::InstantiateMsg {
        admin,
        name: "DAO DAO".to_string(),
        description: "A DAO that makes DAO tooling".to_string(),
        image_url: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: chain
                .orc
                .contract_map
                .code_id("cw20_staked_balance_voting")?,
            msg: to_binary(&cw20_staked_balance_voting::msg::InstantiateMsg {
                token_info: cw20_staked_balance_voting::msg::TokenInfo::New {
                    code_id: chain.orc.contract_map.code_id("cw20_base")?,
                    label: "DAO DAO Gov token".to_string(),
                    name: "DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: user_addr,
                        amount: Uint128::new(100_000_000),
                    }],
                    marketing: None,
                    staking_code_id: chain.orc.contract_map.code_id("cw20_stake")?,
                    unstaking_duration: Some(Duration::Time(1209600)),
                    initial_dao_balance: None,
                },
                active_threshold: None,
            })?,
            admin: Admin::CoreContract {},
            label: "DAO DAO Voting Module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: chain.orc.contract_map.code_id("cw_proposal_single")?,
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
                close_proposal_on_execution_failure: false,
            })?,
            admin: Admin::CoreContract {},
            label: "DAO DAO Proposal Module".to_string(),
        }],
        initial_items: None,
    };

    chain
        .orc
        .instantiate("cw_core", op_name, &msg, &chain.user.key)?;

    let res = chain
        .orc
        .query("cw_core", op_name, &cw_core::msg::QueryMsg::DumpState {})?;

    Ok(DaoState {
        addr: chain.orc.contract_map.address("cw_core")?,
        state: res.data()?,
    })
}
