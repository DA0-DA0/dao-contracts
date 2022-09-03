#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use cw_pre_propose_base::{
    error::PreProposeError,
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const CONTRACT_NAME: &str = "crates.io:cw-pre-propose-base-proposal-single";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, JsonSchema, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProposeMessage {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
    },
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, Empty>;
pub type QueryMsg = QueryBase<Empty>;

/// Internal version of the propose message that includes the
/// `proposer` field. The module will fill this in based on the sender
/// of the external message.
#[derive(Serialize, JsonSchema, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum ProposeMessageInternal {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
        proposer: Option<String>,
    },
}

type PrePropose = PreProposeContract<Empty, Empty, Empty, ProposeMessageInternal>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, PreProposeError> {
    let resp = PrePropose::default().instantiate(deps.branch(), env, info, msg)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, PreProposeError> {
    type ExecuteInternal = ExecuteBase<ProposeMessageInternal, Empty>;
    let internalized = match msg {
        ExecuteMsg::Propose {
            msg:
                ProposeMessage::Propose {
                    title,
                    description,
                    msgs,
                },
        } => ExecuteInternal::Propose {
            msg: ProposeMessageInternal::Propose {
                // Fill in proposer based on message sender.
                proposer: Some(info.sender.to_string()),
                title,
                description,
                msgs,
            },
        },
        ExecuteMsg::Ext { msg } => ExecuteInternal::Ext { msg },
        ExecuteMsg::ProposalHook(hook) => ExecuteInternal::ProposalHook(hook),
    };

    PrePropose::default().execute(deps, env, info, internalized)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    PrePropose::default().query(deps, env, msg)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, to_binary, Addr, Uint128};
    use cps::query::ProposalResponse;
    use cw_core::state::ProposalModule;
    use cw_core_interface::{Admin, ModuleInstantiateInfo};
    use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor};
    use cw_proposal_single as cps;
    use cw_utils::Duration;
    use testing::helpers::instantiate_with_cw4_groups_governance;
    use voting::{
        denom::UncheckedDenom,
        deposit::{DepositRefundPolicy, DepositToken, UncheckedDepositInfo},
        pre_propose::{PreProposeInfo, ProposalCreationPolicy},
        status::Status,
        threshold::{PercentageThreshold, Threshold},
        voting::Vote,
    };

    use super::*;

    fn cw_dao_proposal_single_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cps::contract::execute,
            cps::contract::instantiate,
            cps::contract::query,
        )
        .with_migrate(cps::contract::migrate)
        .with_reply(cps::contract::reply);
        Box::new(contract)
    }

    fn cw_pre_propose_base_proposal_single() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    fn get_default_proposal_module_instantiate(
        app: &mut App,
        deposit_info: Option<UncheckedDepositInfo>,
        open_proposal_submission: bool,
    ) -> cps::msg::InstantiateMsg {
        let pre_propose_id = app.store_code(cw_pre_propose_base_proposal_single());

        cps::msg::InstantiateMsg {
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: Duration::Time(86400),
            min_voting_period: None,
            only_members_execute: false,
            allow_revoting: false,
            pre_propose_info: PreProposeInfo::ModuleMayPropose {
                info: ModuleInstantiateInfo {
                    code_id: pre_propose_id,
                    msg: to_binary(&InstantiateMsg {
                        deposit_info,
                        open_proposal_submission,
                        ext: Empty::default(),
                    })
                    .unwrap(),
                    admin: Some(Admin::Instantiator {}),
                    label: "baby's first pre-propose module".to_string(),
                },
            },
            close_proposal_on_execution_failure: false,
        }
    }

    // A cw4-group based DAO that takes native tokens as proposal
    // deposits.
    #[test]
    fn test_simple_native_playthrough() {
        let mut app = App::default();
        let cps_id = app.store_code(cw_dao_proposal_single_contract());
        let pre_propose_id = app.store_code(cw_pre_propose_base_proposal_single());

        let proposal_module_instantiate = get_default_proposal_module_instantiate(
            &mut app,
            Some(UncheckedDepositInfo {
                denom: DepositToken::Token {
                    denom: UncheckedDenom::Native("ujuno".to_string()),
                },
                amount: Uint128::new(10),
                refund_policy: DepositRefundPolicy::Always,
            }),
            false,
        );

        let core_addr = instantiate_with_cw4_groups_governance(
            &mut app,
            cps_id,
            to_binary(&proposal_module_instantiate).unwrap(),
            Some(vec![
                cw20::Cw20Coin {
                    address: "ekez".to_string(),
                    amount: Uint128::new(9),
                },
                cw20::Cw20Coin {
                    address: "keze".to_string(),
                    amount: Uint128::new(8),
                },
            ]),
        );
        let proposal_modules: Vec<ProposalModule> = app
            .wrap()
            .query_wasm_smart(
                core_addr.clone(),
                &cw_core::msg::QueryMsg::ProposalModules {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(proposal_modules.len(), 1);
        let proposal_single = proposal_modules.into_iter().next().unwrap().address;
        let config: cps::state::Config = app
            .wrap()
            .query_wasm_smart(proposal_single.clone(), &cps::msg::QueryMsg::Config {})
            .unwrap();

        let pre_propose = match config.proposal_creation_policy {
            ProposalCreationPolicy::Module { addr } => addr,
            _ => panic!("expected a module for the proposal creation policy"),
        };

        // Mint some ekez tokens for ekez so we can pay the deposit.
        app.sudo(cw_multi_test::SudoMsg::Bank(BankSudo::Mint {
            to_address: "ekez".to_string(),
            amount: coins(10, "ujuno"),
        }))
        .unwrap();

        app.execute_contract(
            Addr::unchecked("ekez"),
            pre_propose,
            &ExecuteMsg::Propose {
                msg: ProposeMessage::Propose {
                    title: "pre propose works!".to_string(),
                    description: "wow..".to_string(),
                    msgs: vec![],
                },
            },
            &coins(10, "ujuno"),
        )
        .unwrap();

        let proposal: ProposalResponse = app
            .wrap()
            .query_wasm_smart(
                proposal_single.clone(),
                &cps::msg::QueryMsg::Proposal { proposal_id: 1 },
            )
            .unwrap();

        assert_eq!(proposal.proposal.title, "pre propose works!".to_string());
        assert_eq!(proposal.proposal.description, "wow..".to_string());
        assert_eq!(proposal.proposal.msgs, vec![]);

        // Vote no on the proposal. This should cause it to be rejected.
        app.execute_contract(
            Addr::unchecked("ekez"),
            proposal_single.clone(),
            &cps::msg::ExecuteMsg::Vote {
                proposal_id: 1,
                vote: Vote::No,
            },
            &[],
        )
        .unwrap();

        let proposal: ProposalResponse = app
            .wrap()
            .query_wasm_smart(
                proposal_single.clone(),
                &cps::msg::QueryMsg::Proposal { proposal_id: 1 },
            )
            .unwrap();
        assert_eq!(proposal.proposal.status, Status::Rejected);

        // Close the proposal. This should cause the deposit to be
        // refunded.
        app.execute_contract(
            Addr::unchecked("ekez"),
            proposal_single,
            &cps::msg::ExecuteMsg::Close { proposal_id: 1 },
            &[],
        )
        .unwrap();

        let balance = app
            .wrap()
            .query_balance(Addr::unchecked("ekez"), "ujuno")
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(10))
    }
}
