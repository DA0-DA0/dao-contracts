use super::chain::Chain;
use anyhow::Result;
use cosm_orc::orchestrator::SigningKey;
use cosmwasm_std::{to_binary, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_utils::Duration;
use cwd_core::query::DumpStateResponse;
use cwd_interface::{Admin, ModuleInstantiateInfo};
use cwd_voting::{
    deposit::{DepositRefundPolicy, DepositToken, UncheckedDepositInfo},
    pre_propose::PreProposeInfo,
    threshold::PercentageThreshold,
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
    key: &SigningKey,
) -> Result<DaoState> {
    let msg = cwd_core::msg::InstantiateMsg {
        dao_uri: None,
        admin,
        name: "DAO DAO".to_string(),
        description: "A DAO that makes DAO tooling".to_string(),
        image_url: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: chain.orc.contract_map.code_id("cwd_voting_cw20_staked")?,
            msg: to_binary(&cwd_voting_cw20_staked::msg::InstantiateMsg {
                token_info: cwd_voting_cw20_staked::msg::TokenInfo::New {
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
            admin: Some(Admin::CoreModule {}),
            label: "DAO DAO Voting Module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: chain.orc.contract_map.code_id("cwd_proposal_single")?,
            msg: to_binary(&cwd_proposal_single::msg::InstantiateMsg {
                min_voting_period: None,
                threshold: Threshold::ThresholdQuorum {
                    threshold: PercentageThreshold::Majority {},
                    quorum: PercentageThreshold::Percent(Decimal::percent(35)),
                },
                max_voting_period: Duration::Time(432000),
                allow_revoting: false,
                only_members_execute: true,
                close_proposal_on_execution_failure: false,
                pre_propose_info: PreProposeInfo::ModuleMayPropose {
                    info: ModuleInstantiateInfo {
                        code_id: chain.orc.contract_map.code_id("cwd_pre_propose_single")?,
                        msg: to_binary(&cwd_pre_propose_single::InstantiateMsg {
                            deposit_info: Some(UncheckedDepositInfo {
                                denom: DepositToken::VotingModuleToken {},
                                amount: Uint128::new(1000000000),
                                refund_policy: DepositRefundPolicy::OnlyPassed,
                            }),
                            open_proposal_submission: false,
                            extension: Empty::default(),
                        })
                        .unwrap(),
                        admin: Some(Admin::CoreModule {}),
                        label: "DAO DAO Pre-Propose Module".to_string(),
                    },
                },
            })?,
            admin: Some(Admin::CoreModule {}),
            label: "DAO DAO Proposal Module".to_string(),
        }],
        initial_items: None,
    };

    chain
        .orc
        .instantiate("cwd_core", op_name, &msg, key, None, vec![])?;

    let res = chain
        .orc
        .query("cwd_core", &cwd_core::msg::QueryMsg::DumpState {})?;

    Ok(DaoState {
        addr: chain.orc.contract_map.address("cwd_core")?,
        state: res.data()?,
    })
}
