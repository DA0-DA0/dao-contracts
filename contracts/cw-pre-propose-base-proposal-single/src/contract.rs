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
    use cosmwasm_std::{coins, from_slice, to_binary, Addr, Attribute, Coin, Event, Uint128};
    use cps::query::ProposalResponse;
    use cw2::ContractVersion;
    use cw20::Cw20Coin;
    use cw_core::state::ProposalModule;
    use cw_core_interface::{Admin, ModuleInstantiateInfo};
    use cw_denom::UncheckedDenom;
    use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor};
    use cw_proposal_single as cps;
    use cw_utils::Duration;
    use proposal_hooks::ProposalHookMsg;
    use testing::helpers::instantiate_with_cw4_groups_governance;
    use voting::{
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

    fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
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

    fn instantiate_cw20_base_default(app: &mut App) -> Addr {
        let cw20_id = app.store_code(cw20_base_contract());
        let cw20_instantiate = cw20_base::msg::InstantiateMsg {
            name: "cw20 token".to_string(),
            symbol: "cwtwenty".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: "ekez".to_string(),
                amount: Uint128::new(10),
            }],
            mint: None,
            marketing: None,
        };
        app.instantiate_contract(
            cw20_id,
            Addr::unchecked("ekez"),
            &cw20_instantiate,
            &[],
            "cw20-base",
            None,
        )
        .unwrap()
    }

    struct DefaultTestSetup {
        core_addr: Addr,
        proposal_single: Addr,
        pre_propose: Addr,
    }
    fn setup_default_test(
        app: &mut App,
        deposit_info: Option<UncheckedDepositInfo>,
        open_proposal_submission: bool,
    ) -> DefaultTestSetup {
        let cps_id = app.store_code(cw_dao_proposal_single_contract());

        let proposal_module_instantiate =
            get_default_proposal_module_instantiate(app, deposit_info, open_proposal_submission);

        let core_addr = instantiate_with_cw4_groups_governance(
            app,
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

        DefaultTestSetup {
            core_addr,
            proposal_single,
            pre_propose,
        }
    }

    fn make_proposal(
        app: &mut App,
        pre_propose: Addr,
        proposal_module: Addr,
        proposer: &str,
        funds: &[Coin],
    ) -> u64 {
        let res = app
            .execute_contract(
                Addr::unchecked(proposer),
                pre_propose,
                &ExecuteMsg::Propose {
                    msg: ProposeMessage::Propose {
                        title: "title".to_string(),
                        description: "description".to_string(),
                        msgs: vec![],
                    },
                },
                funds,
            )
            .unwrap();

        // The new proposal hook is the last message that fires in
        // this process so we get the proposal ID from it's
        // attributes. We could do this by looking at the proposal
        // creation attributes but this changes relative position
        // depending on if a cw20 or native deposit is being used.
        let attrs = res.custom_attrs(res.events.len() - 1);
        let id = attrs[attrs.len() - 1].value.parse().unwrap();
        let proposal: ProposalResponse = app
            .wrap()
            .query_wasm_smart(
                proposal_module,
                &cps::msg::QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap();

        assert_eq!(proposal.proposal.proposer, Addr::unchecked(proposer));
        assert_eq!(proposal.proposal.title, "title".to_string());
        assert_eq!(proposal.proposal.description, "description".to_string());
        assert_eq!(proposal.proposal.msgs, vec![]);

        id
    }

    fn mint_natives(app: &mut App, receiver: &str, coins: Vec<Coin>) {
        // Mint some ekez tokens for ekez so we can pay the deposit.
        app.sudo(cw_multi_test::SudoMsg::Bank(BankSudo::Mint {
            to_address: receiver.to_string(),
            amount: coins,
        }))
        .unwrap();
    }

    fn increase_allowance(
        app: &mut App,
        sender: &str,
        receiver: &Addr,
        cw20: Addr,
        amount: Uint128,
    ) {
        app.execute_contract(
            Addr::unchecked(sender),
            cw20,
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: receiver.to_string(),
                amount,
                expires: None,
            },
            &[],
        )
        .unwrap();
    }

    fn get_balance_cw20<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = cw20::Cw20QueryMsg::Balance {
            address: address.into(),
        };
        let result: cw20::BalanceResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    fn vote(app: &mut App, module: Addr, sender: &str, id: u64, position: Vote) -> Status {
        app.execute_contract(
            Addr::unchecked(sender),
            module.clone(),
            &cps::msg::ExecuteMsg::Vote {
                proposal_id: id,
                vote: position,
            },
            &[],
        )
        .unwrap();

        let proposal: ProposalResponse = app
            .wrap()
            .query_wasm_smart(module, &cps::msg::QueryMsg::Proposal { proposal_id: id })
            .unwrap();

        proposal.proposal.status
    }

    fn close_proposal(app: &mut App, module: Addr, sender: &str, proposal_id: u64) {
        app.execute_contract(
            Addr::unchecked(sender),
            module,
            &cps::msg::ExecuteMsg::Close { proposal_id },
            &[],
        )
        .unwrap();
    }

    fn execute_proposal(app: &mut App, module: Addr, sender: &str, proposal_id: u64) {
        app.execute_contract(
            Addr::unchecked(sender),
            module,
            &cps::msg::ExecuteMsg::Execute { proposal_id },
            &[],
        )
        .unwrap();
    }

    enum EndStatus {
        Passed,
        Failed,
    }
    enum RefundReceiver {
        Proposer,
        Dao,
    }

    fn test_native_permutation(
        end_status: EndStatus,
        refund_policy: DepositRefundPolicy,
        receiver: RefundReceiver,
    ) {
        let mut app = App::default();

        let DefaultTestSetup {
            core_addr,
            proposal_single,
            pre_propose,
        } = setup_default_test(
            &mut app,
            Some(UncheckedDepositInfo {
                denom: DepositToken::Token {
                    denom: UncheckedDenom::Native("ujuno".to_string()),
                },
                amount: Uint128::new(10),
                refund_policy,
            }),
            false,
        );

        mint_natives(&mut app, "ekez", coins(10, "ujuno"));
        let id = make_proposal(
            &mut app,
            pre_propose.clone(),
            proposal_single.clone(),
            "ekez",
            &coins(10, "ujuno"),
        );
        let (position, expected_status, trigger_refund): (
            _,
            _,
            fn(&mut App, Addr, &str, u64) -> (),
        ) = match end_status {
            EndStatus::Passed => (Vote::Yes, Status::Passed, execute_proposal),
            EndStatus::Failed => (Vote::No, Status::Rejected, close_proposal),
        };
        let new_status = vote(&mut app, proposal_single.clone(), "ekez", id, position);
        assert_eq!(new_status, expected_status);

        // Close or execute the proposal to trigger a refund.
        trigger_refund(&mut app, proposal_single, "ekez", id);

        let (dao_expected, proposer_expected) = match receiver {
            RefundReceiver::Proposer => (0, 10),
            RefundReceiver::Dao => (10, 0),
        };

        let proposer_balance = app
            .wrap()
            .query_balance(Addr::unchecked("ekez"), "ujuno")
            .unwrap();
        let dao_balance = app.wrap().query_balance(core_addr, "ujuno").unwrap();
        assert_eq!(proposer_expected, proposer_balance.amount.u128());
        assert_eq!(dao_expected, dao_balance.amount.u128())
    }

    fn test_cw20_permutation(
        end_status: EndStatus,
        refund_policy: DepositRefundPolicy,
        receiver: RefundReceiver,
    ) {
        let mut app = App::default();

        let cw20_address = instantiate_cw20_base_default(&mut app);

        let DefaultTestSetup {
            core_addr,
            proposal_single,
            pre_propose,
        } = setup_default_test(
            &mut app,
            Some(UncheckedDepositInfo {
                denom: DepositToken::Token {
                    denom: UncheckedDenom::Cw20(cw20_address.to_string()),
                },
                amount: Uint128::new(10),
                refund_policy,
            }),
            false,
        );

        increase_allowance(
            &mut app,
            "ekez",
            &pre_propose,
            cw20_address.clone(),
            Uint128::new(10),
        );
        let id = make_proposal(
            &mut app,
            pre_propose.clone(),
            proposal_single.clone(),
            "ekez",
            &[],
        );
        let (position, expected_status, trigger_refund): (
            _,
            _,
            fn(&mut App, Addr, &str, u64) -> (),
        ) = match end_status {
            EndStatus::Passed => (Vote::Yes, Status::Passed, execute_proposal),
            EndStatus::Failed => (Vote::No, Status::Rejected, close_proposal),
        };
        let new_status = vote(&mut app, proposal_single.clone(), "ekez", id, position);
        assert_eq!(new_status, expected_status);

        // Close or execute the proposal to trigger a refund.
        trigger_refund(&mut app, proposal_single, "ekez", id);

        let (dao_expected, proposer_expected) = match receiver {
            RefundReceiver::Proposer => (0, 10),
            RefundReceiver::Dao => (10, 0),
        };

        let proposer_balance = get_balance_cw20(&app, &cw20_address, "ekez");
        let dao_balance = get_balance_cw20(&app, &cw20_address, &core_addr);
        assert_eq!(proposer_expected, proposer_balance.u128());
        assert_eq!(dao_expected, dao_balance.u128())
    }

    #[test]
    fn test_native_failed_always_refund() {
        test_native_permutation(
            EndStatus::Failed,
            DepositRefundPolicy::Always,
            RefundReceiver::Proposer,
        )
    }
    #[test]
    fn test_cw20_failed_always_refund() {
        test_cw20_permutation(
            EndStatus::Failed,
            DepositRefundPolicy::Always,
            RefundReceiver::Proposer,
        )
    }

    #[test]
    fn test_native_passed_always_refund() {
        test_native_permutation(
            EndStatus::Passed,
            DepositRefundPolicy::Always,
            RefundReceiver::Proposer,
        )
    }
    #[test]
    fn test_cw20_passed_always_refund() {
        test_cw20_permutation(
            EndStatus::Passed,
            DepositRefundPolicy::Always,
            RefundReceiver::Proposer,
        )
    }

    #[test]
    fn test_native_passed_never_refund() {
        test_native_permutation(
            EndStatus::Passed,
            DepositRefundPolicy::Never,
            RefundReceiver::Dao,
        )
    }
    #[test]
    fn test_cw20_passed_never_refund() {
        test_cw20_permutation(
            EndStatus::Passed,
            DepositRefundPolicy::Never,
            RefundReceiver::Dao,
        )
    }

    #[test]
    fn test_native_failed_never_refund() {
        test_native_permutation(
            EndStatus::Failed,
            DepositRefundPolicy::Never,
            RefundReceiver::Dao,
        )
    }
    #[test]
    fn test_cw20_failed_never_refund() {
        test_cw20_permutation(
            EndStatus::Failed,
            DepositRefundPolicy::Never,
            RefundReceiver::Dao,
        )
    }

    #[test]
    fn test_native_passed_passed_refund() {
        test_native_permutation(
            EndStatus::Passed,
            DepositRefundPolicy::OnlyPassed,
            RefundReceiver::Proposer,
        )
    }
    #[test]
    fn test_cw20_passed_passed_refund() {
        test_cw20_permutation(
            EndStatus::Passed,
            DepositRefundPolicy::OnlyPassed,
            RefundReceiver::Proposer,
        )
    }

    #[test]
    fn test_native_failed_passed_refund() {
        test_native_permutation(
            EndStatus::Failed,
            DepositRefundPolicy::OnlyPassed,
            RefundReceiver::Dao,
        )
    }
    #[test]
    fn test_cw20_failed_passed_refund() {
        test_cw20_permutation(
            EndStatus::Failed,
            DepositRefundPolicy::OnlyPassed,
            RefundReceiver::Dao,
        )
    }

    #[test]
    fn test_set_version() {
        let mut app = App::default();

        let DefaultTestSetup {
            core_addr: _,
            proposal_single: _,
            pre_propose,
        } = setup_default_test(
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

        let info: ContractVersion = from_slice(
            &app.wrap()
                .query_wasm_raw(pre_propose, "contract_info".as_bytes())
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string()
            },
            info
        )
    }

    #[test]
    fn test_permissions() {
        let mut app = App::default();

        let DefaultTestSetup {
            core_addr,
            proposal_single: _,
            pre_propose,
        } = setup_default_test(
            &mut app,
            Some(UncheckedDepositInfo {
                denom: DepositToken::Token {
                    denom: UncheckedDenom::Native("ujuno".to_string()),
                },
                amount: Uint128::new(10),
                refund_policy: DepositRefundPolicy::Always,
            }),
            false, // no open proposal submission.
        );

        let err: PreProposeError = app
            .execute_contract(
                Addr::unchecked("notmodule"),
                pre_propose.clone(),
                &ExecuteMsg::ProposalHook(ProposalHookMsg::NewProposal {
                    id: 1,
                    proposer: "ekez".to_string(),
                }),
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, PreProposeError::NotModule {});

        let err: PreProposeError = app
            .execute_contract(
                core_addr,
                pre_propose.clone(),
                &ExecuteMsg::ProposalHook(ProposalHookMsg::ProposalStatusChanged {
                    id: 1,
                    old_status: "rejected".to_string(),
                    new_status: "closed".to_string(),
                }),
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, PreProposeError::NotModule {});

        // Non-members may not propose when open_propose_submission is
        // disabled.
        let err: PreProposeError = app
            .execute_contract(
                Addr::unchecked("nonmember"),
                pre_propose,
                &ExecuteMsg::Propose {
                    msg: ProposeMessage::Propose {
                        title: "I would like to join the DAO".to_string(),
                        description: "though, I am currently not a member.".to_string(),
                        msgs: vec![],
                    },
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, PreProposeError::NotMember {})
    }

    #[test]
    fn test_propose_open_proposal_submission() {
        let mut app = App::default();
        let DefaultTestSetup {
            core_addr: _,
            proposal_single,
            pre_propose,
        } = setup_default_test(
            &mut app,
            Some(UncheckedDepositInfo {
                denom: DepositToken::Token {
                    denom: UncheckedDenom::Native("ujuno".to_string()),
                },
                amount: Uint128::new(10),
                refund_policy: DepositRefundPolicy::Always,
            }),
            true, // yes, open proposal submission.
        );

        // Non-member proposes.
        mint_natives(&mut app, "nonmember", coins(10, "ujuno"));
        let id = make_proposal(
            &mut app,
            pre_propose,
            proposal_single.clone(),
            "nonmember",
            &coins(10, "ujuno"),
        );
        // Member votes.
        let new_status = vote(&mut app, proposal_single, "ekez", id, Vote::Yes);
        assert_eq!(Status::Passed, new_status)
    }

    #[test]
    fn test_no_deposit_required_open_submission() {
        let mut app = App::default();
        let DefaultTestSetup {
            core_addr: _,
            proposal_single,
            pre_propose,
        } = setup_default_test(
            &mut app, None, true, // yes, open proposal submission.
        );

        // Non-member proposes.
        let id = make_proposal(
            &mut app,
            pre_propose,
            proposal_single.clone(),
            "nonmember",
            &[],
        );
        // Member votes.
        let new_status = vote(&mut app, proposal_single, "ekez", id, Vote::Yes);
        assert_eq!(Status::Passed, new_status)
    }

    #[test]
    fn test_no_deposit_required_members_submission() {
        let mut app = App::default();
        let DefaultTestSetup {
            core_addr: _,
            proposal_single,
            pre_propose,
        } = setup_default_test(
            &mut app, None, false, // no open proposal submission.
        );

        // Non-member proposes and this fails.
        let err: PreProposeError = app
            .execute_contract(
                Addr::unchecked("nonmember"),
                pre_propose.clone(),
                &ExecuteMsg::Propose {
                    msg: ProposeMessage::Propose {
                        title: "I would like to join the DAO".to_string(),
                        description: "though, I am currently not a member.".to_string(),
                        msgs: vec![],
                    },
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, PreProposeError::NotMember {});

        let id = make_proposal(&mut app, pre_propose, proposal_single.clone(), "ekez", &[]);
        let new_status = vote(&mut app, proposal_single, "ekez", id, Vote::Yes);
        assert_eq!(Status::Passed, new_status)
    }

    #[test]
    fn test_execute_extension_does_nothing() {
        let mut app = App::default();
        let DefaultTestSetup {
            core_addr: _,
            proposal_single,
            pre_propose,
        } = setup_default_test(
            &mut app, None, false, // no open proposal submission.
        );

        let res = app
            .execute_contract(
                Addr::unchecked("ekez"),
                pre_propose,
                &ExecuteMsg::Ext {
                    msg: Empty::default(),
                },
                &[],
            )
            .unwrap();

        // There should be one event which is the invocation of the contract.
        assert_eq!(res.events.len(), 1);
        assert_eq!(res.events[0].ty, "execute".to_string());
        assert_eq!(res.events[0].attributes.len(), 1);
        assert_eq!(
            res.events[0].attributes[0].key,
            "_contract_addr".to_string()
        )
    }

    #[test]
    #[should_panic(expected = "invalid zero deposit. set the deposit to `None` to have no deposit")]
    fn test_instantiate_with_zero_native_deposit() {
        let mut app = App::default();

        let cps_id = app.store_code(cw_dao_proposal_single_contract());

        let proposal_module_instantiate = {
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
                            deposit_info: Some(UncheckedDepositInfo {
                                denom: DepositToken::Token {
                                    denom: UncheckedDenom::Native("ujuno".to_string()),
                                },
                                amount: Uint128::zero(),
                                refund_policy: DepositRefundPolicy::OnlyPassed,
                            }),
                            open_proposal_submission: false,
                            ext: Empty::default(),
                        })
                        .unwrap(),
                        admin: Some(Admin::Instantiator {}),
                        label: "baby's first pre-propose module".to_string(),
                    },
                },
                close_proposal_on_execution_failure: false,
            }
        };

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
    }

    #[test]
    #[should_panic(expected = "invalid zero deposit. set the deposit to `None` to have no deposit")]
    fn test_instantiate_with_zero_cw20_deposit() {
        let mut app = App::default();

        let cw20_addr = instantiate_cw20_base_default(&mut app);

        let cps_id = app.store_code(cw_dao_proposal_single_contract());

        let proposal_module_instantiate = {
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
                            deposit_info: Some(UncheckedDepositInfo {
                                denom: DepositToken::Token {
                                    denom: UncheckedDenom::Cw20(cw20_addr.into_string()),
                                },
                                amount: Uint128::zero(),
                                refund_policy: DepositRefundPolicy::OnlyPassed,
                            }),
                            open_proposal_submission: false,
                            ext: Empty::default(),
                        })
                        .unwrap(),
                        admin: Some(Admin::Instantiator {}),
                        label: "baby's first pre-propose module".to_string(),
                    },
                },
                close_proposal_on_execution_failure: false,
            }
        };

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
    }
}
