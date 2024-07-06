use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    SubMsg, WasmMsg,
};

use cw2::set_contract_version;

use cw_denom::UncheckedDenom;
use dao_interface::voting::{Query as CwCoreQuery, VotingPowerAtHeightResponse};
use dao_voting::{
    deposit::{DepositRefundPolicy, UncheckedDepositInfo},
    pre_propose::{PreProposeSubmissionPolicy, PreProposeSubmissionPolicyError},
    status::Status,
};
use serde::Serialize;

use crate::{
    error::PreProposeError,
    msg::{DepositInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{Config, PreProposeContract},
};

const CONTRACT_NAME: &str = "crates.io::dao-pre-propose-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
    PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
where
    ProposalMessage: Serialize,
    QueryExt: JsonSchema,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: InstantiateMsg<InstantiateExt>,
    ) -> Result<Response, PreProposeError> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // The proposal module instantiates us. We're
        // making limited assumptions here. The only way to associate
        // a deposit module with a proposal module is for the proposal
        // module to instantiate it.
        self.proposal_module.save(deps.storage, &info.sender)?;

        // Query the proposal module for its DAO.
        let dao: Addr = deps
            .querier
            .query_wasm_smart(info.sender.clone(), &CwCoreQuery::Dao {})?;

        self.dao.save(deps.storage, &dao)?;

        let deposit_info = msg
            .deposit_info
            .map(|info| info.into_checked(deps.as_ref(), dao.clone()))
            .transpose()?;

        msg.submission_policy.validate()?;

        let config = Config {
            deposit_info,
            submission_policy: msg.submission_policy,
        };

        self.config.save(deps.storage, &config)?;

        Ok(Response::default()
            .add_attribute("method", "instantiate")
            .add_attribute("proposal_module", info.sender.into_string())
            .add_attribute("deposit_info", format!("{:?}", config.deposit_info))
            .add_attribute(
                "submission_policy",
                config.submission_policy.human_readable(),
            )
            .add_attribute("dao", dao))
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<ProposalMessage, ExecuteExt>,
    ) -> Result<Response, PreProposeError> {
        match msg {
            ExecuteMsg::Propose { msg } => self.execute_propose(deps, env, info, msg),
            ExecuteMsg::UpdateConfig {
                deposit_info,
                submission_policy,
            } => self.execute_update_config(deps, info, deposit_info, submission_policy),
            ExecuteMsg::UpdateSubmissionPolicy {
                denylist_add,
                denylist_remove,
                set_dao_members,
                allowlist_add,
                allowlist_remove,
            } => self.execute_update_submission_policy(
                deps,
                info,
                denylist_add,
                denylist_remove,
                set_dao_members,
                allowlist_add,
                allowlist_remove,
            ),
            ExecuteMsg::Withdraw { denom } => {
                self.execute_withdraw(deps.as_ref(), env, info, denom)
            }
            ExecuteMsg::AddProposalSubmittedHook { address } => {
                self.execute_add_proposal_submitted_hook(deps, info, address)
            }
            ExecuteMsg::RemoveProposalSubmittedHook { address } => {
                self.execute_remove_proposal_submitted_hook(deps, info, address)
            }
            ExecuteMsg::ProposalCompletedHook {
                proposal_id,
                new_status,
            } => self.execute_proposal_completed_hook(deps.as_ref(), info, proposal_id, new_status),

            ExecuteMsg::Extension { .. } => Ok(Response::default()),
        }
    }

    pub fn execute_propose(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ProposalMessage,
    ) -> Result<Response, PreProposeError> {
        self.check_can_submit(deps.as_ref(), info.sender.clone())?;

        let config = self.config.load(deps.storage)?;

        let deposit_messages = if let Some(ref deposit_info) = config.deposit_info {
            deposit_info.check_native_deposit_paid(&info)?;
            deposit_info.get_take_deposit_messages(&info.sender, &env.contract.address)?
        } else {
            vec![]
        };

        let proposal_module = self.proposal_module.load(deps.storage)?;

        // Snapshot the deposit using the ID of the proposal that we
        // will create.
        let next_id = deps.querier.query_wasm_smart(
            &proposal_module,
            &dao_interface::proposal::Query::NextProposalId {},
        )?;
        self.deposits.save(
            deps.storage,
            next_id,
            &(config.deposit_info, info.sender.clone()),
        )?;

        let propose_messsage = WasmMsg::Execute {
            contract_addr: proposal_module.into_string(),
            msg: to_json_binary(&msg)?,
            funds: vec![],
        };

        let hooks_msgs = self
            .proposal_submitted_hooks
            .prepare_hooks(deps.storage, |a| {
                let execute = WasmMsg::Execute {
                    contract_addr: a.into_string(),
                    msg: to_json_binary(&msg)?,
                    funds: vec![],
                };
                Ok(SubMsg::new(execute))
            })?;

        Ok(Response::default()
            .add_attribute("method", "execute_propose")
            .add_attribute("sender", info.sender)
            // It's important that the propose message is
            // first. Otherwise, a hook receiver could create a
            // proposal before us and invalidate our `NextProposalId
            // {}` query.
            .add_message(propose_messsage)
            .add_submessages(hooks_msgs)
            .add_messages(deposit_messages))
    }

    pub fn execute_update_config(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        deposit_info: Option<UncheckedDepositInfo>,
        submission_policy: Option<PreProposeSubmissionPolicy>,
    ) -> Result<Response, PreProposeError> {
        let dao = self.dao.load(deps.storage)?;
        if info.sender != dao {
            return Err(PreProposeError::NotDao {});
        }

        let deposit_info = deposit_info
            .map(|d| d.into_checked(deps.as_ref(), dao))
            .transpose()?;

        self.config
            .update(deps.storage, |prev| -> Result<Config, PreProposeError> {
                let new_submission_policy = if let Some(submission_policy) = submission_policy {
                    submission_policy.validate()?;
                    submission_policy
                } else {
                    prev.submission_policy
                };

                Ok(Config {
                    deposit_info,
                    submission_policy: new_submission_policy,
                })
            })?;

        Ok(Response::default()
            .add_attribute("method", "update_config")
            .add_attribute("sender", info.sender))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_update_submission_policy(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        denylist_add: Option<Vec<String>>,
        denylist_remove: Option<Vec<String>>,
        set_dao_members: Option<bool>,
        allowlist_add: Option<Vec<String>>,
        allowlist_remove: Option<Vec<String>>,
    ) -> Result<Response, PreProposeError> {
        let dao = self.dao.load(deps.storage)?;
        if info.sender != dao {
            return Err(PreProposeError::NotDao {});
        }

        let mut config = self.config.load(deps.storage)?;

        match config.submission_policy {
            PreProposeSubmissionPolicy::Anyone { denylist } => {
                // Error if other values that apply to Specific were set.
                if set_dao_members.is_some()
                    || allowlist_add.is_some()
                    || allowlist_remove.is_some()
                {
                    return Err(PreProposeError::SubmissionPolicy(
                        PreProposeSubmissionPolicyError::AnyoneInvalidUpdateFields {},
                    ));
                }

                let mut denylist = denylist.unwrap_or_default();

                // Add to denylist.
                if let Some(mut denylist_add) = denylist_add {
                    // Validate addresses.
                    denylist_add
                        .iter()
                        .map(|addr| deps.api.addr_validate(addr))
                        .collect::<StdResult<Vec<Addr>>>()?;

                    denylist.append(&mut denylist_add);
                    denylist.dedup();
                }

                // Remove from denylist.
                if let Some(denylist_remove) = denylist_remove {
                    // Validate addresses.
                    denylist_remove
                        .iter()
                        .map(|addr| deps.api.addr_validate(addr))
                        .collect::<StdResult<Vec<Addr>>>()?;

                    denylist.retain(|a| !denylist_remove.contains(a));
                }

                let denylist = if denylist.is_empty() {
                    None
                } else {
                    Some(denylist)
                };

                config.submission_policy = PreProposeSubmissionPolicy::Anyone { denylist };
            }
            PreProposeSubmissionPolicy::Specific {
                dao_members,
                allowlist,
                denylist,
            } => {
                let dao_members = if let Some(new_dao_members) = set_dao_members {
                    new_dao_members
                } else {
                    dao_members
                };

                let mut allowlist = allowlist.unwrap_or_default();
                let mut denylist = denylist.unwrap_or_default();

                // Add to allowlist.
                if let Some(mut allowlist_add) = allowlist_add {
                    // Validate addresses.
                    allowlist_add
                        .iter()
                        .map(|addr| deps.api.addr_validate(addr))
                        .collect::<StdResult<Vec<Addr>>>()?;

                    allowlist.append(&mut allowlist_add);
                    allowlist.dedup();
                }

                // Remove from allowlist.
                if let Some(allowlist_remove) = allowlist_remove {
                    // Validate addresses.
                    allowlist_remove
                        .iter()
                        .map(|addr| deps.api.addr_validate(addr))
                        .collect::<StdResult<Vec<Addr>>>()?;

                    allowlist.retain(|a| !allowlist_remove.contains(a));
                }

                // Add to denylist.
                if let Some(mut denylist_add) = denylist_add {
                    // Validate addresses.
                    denylist_add
                        .iter()
                        .map(|addr| deps.api.addr_validate(addr))
                        .collect::<StdResult<Vec<Addr>>>()?;

                    denylist.append(&mut denylist_add);
                    denylist.dedup();
                }

                // Remove from denylist.
                if let Some(denylist_remove) = denylist_remove {
                    // Validate addresses.
                    denylist_remove
                        .iter()
                        .map(|addr| deps.api.addr_validate(addr))
                        .collect::<StdResult<Vec<Addr>>>()?;

                    denylist.retain(|a| !denylist_remove.contains(a));
                }

                // Replace empty vectors with None.
                let allowlist = if allowlist.is_empty() {
                    None
                } else {
                    Some(allowlist)
                };
                let denylist = if denylist.is_empty() {
                    None
                } else {
                    Some(denylist)
                };

                config.submission_policy = PreProposeSubmissionPolicy::Specific {
                    dao_members,
                    allowlist,
                    denylist,
                };
            }
        }

        config.submission_policy.validate()?;
        self.config.save(deps.storage, &config)?;

        Ok(Response::default()
            .add_attribute("method", "update_submission_policy")
            .add_attribute("sender", info.sender))
    }

    pub fn execute_withdraw(
        &self,
        deps: Deps,
        env: Env,
        info: MessageInfo,
        denom: Option<UncheckedDenom>,
    ) -> Result<Response, PreProposeError> {
        let dao = self.dao.load(deps.storage)?;
        if info.sender != dao {
            Err(PreProposeError::NotDao {})
        } else {
            let denom = match denom {
                Some(denom) => Some(denom.into_checked(deps)?),
                None => {
                    let config = self.config.load(deps.storage)?;
                    config.deposit_info.map(|d| d.denom)
                }
            };
            match denom {
                None => Err(PreProposeError::NoWithdrawalDenom {}),
                Some(denom) => {
                    let balance = denom.query_balance(&deps.querier, &env.contract.address)?;
                    if balance.is_zero() {
                        Err(PreProposeError::NothingToWithdraw {})
                    } else {
                        let withdraw_message = denom.get_transfer_to_message(&dao, balance)?;
                        Ok(Response::default()
                            .add_message(withdraw_message)
                            .add_attribute("method", "withdraw")
                            .add_attribute("receiver", &dao)
                            .add_attribute("denom", denom.to_string()))
                    }
                }
            }
        }
    }

    pub fn execute_add_proposal_submitted_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, PreProposeError> {
        let dao = self.dao.load(deps.storage)?;
        if info.sender != dao {
            return Err(PreProposeError::NotDao {});
        }

        let addr = deps.api.addr_validate(&address)?;
        self.proposal_submitted_hooks.add_hook(deps.storage, addr)?;

        Ok(Response::default())
    }

    pub fn execute_remove_proposal_submitted_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, PreProposeError> {
        let dao = self.dao.load(deps.storage)?;
        if info.sender != dao {
            return Err(PreProposeError::NotDao {});
        }

        // Validate address
        let addr = deps.api.addr_validate(&address)?;

        // Remove the hook
        self.proposal_submitted_hooks
            .remove_hook(deps.storage, addr)?;

        Ok(Response::default())
    }

    pub fn execute_proposal_completed_hook(
        &self,
        deps: Deps,
        info: MessageInfo,
        id: u64,
        new_status: Status,
    ) -> Result<Response, PreProposeError> {
        let proposal_module = self.proposal_module.load(deps.storage)?;
        if info.sender != proposal_module {
            return Err(PreProposeError::NotModule {});
        }

        // If we receive a proposal completed hook from a proposal
        // module, and it is not in one of these states, something
        // bizare has happened. In that event, this message errors
        // which ought to cause the proposal module to remove this
        // module and open proposal submission to anyone.
        if new_status != Status::Closed
            && new_status != Status::Executed
            && new_status != Status::Vetoed
        {
            return Err(PreProposeError::NotCompleted { status: new_status });
        }

        match self.deposits.may_load(deps.storage, id)? {
            Some((deposit_info, proposer)) => {
                let messages = if let Some(ref deposit_info) = deposit_info {
                    // Determine if refund can be issued
                    let should_refund_to_proposer =
                        match (new_status, deposit_info.clone().refund_policy) {
                            // If policy is refund only passed props, refund for executed status
                            (Status::Executed, DepositRefundPolicy::OnlyPassed) => true,
                            // Don't refund other statuses for OnlyPassed policy
                            (_, DepositRefundPolicy::OnlyPassed) => false,
                            // Refund if the refund policy is always refund
                            (_, DepositRefundPolicy::Always) => true,
                            // Don't refund if the refund is never refund
                            (_, DepositRefundPolicy::Never) => false,
                        };

                    if should_refund_to_proposer {
                        deposit_info.get_return_deposit_message(&proposer)?
                    } else {
                        // If the proposer doesn't get the deposit, the DAO does.
                        let dao = self.dao.load(deps.storage)?;
                        deposit_info.get_return_deposit_message(&dao)?
                    }
                } else {
                    // No deposit info for this proposal. Nothing to do.
                    vec![]
                };

                Ok(Response::default()
                    .add_attribute("method", "execute_proposal_completed_hook")
                    .add_attribute("proposal", id.to_string())
                    .add_attribute("deposit_info", to_json_binary(&deposit_info)?.to_string())
                    .add_messages(messages))
            }

            // If we do not have a deposit for this proposal it was
            // likely created before we were added to the proposal
            // module. In that case, it's not our problem and we just
            // do nothing.
            None => Ok(Response::default()
                .add_attribute("method", "execute_proposal_completed_hook")
                .add_attribute("proposal", id.to_string())),
        }
    }

    pub fn check_can_submit(&self, deps: Deps, who: Addr) -> Result<(), PreProposeError> {
        let config = self.config.load(deps.storage)?;

        match config.submission_policy {
            PreProposeSubmissionPolicy::Anyone { denylist } => {
                if !denylist.unwrap_or_default().contains(&who.to_string()) {
                    return Ok(());
                }
            }
            PreProposeSubmissionPolicy::Specific {
                dao_members,
                allowlist,
                denylist,
            } => {
                // denylist overrides all other settings
                if !denylist.unwrap_or_default().contains(&who.to_string()) {
                    // if on the allowlist, return early
                    if allowlist.unwrap_or_default().contains(&who.to_string()) {
                        return Ok(());
                    }

                    // check DAO membership only if not on the allowlist
                    if dao_members {
                        let dao = self.dao.load(deps.storage)?;
                        let voting_power: VotingPowerAtHeightResponse =
                            deps.querier.query_wasm_smart(
                                dao.into_string(),
                                &CwCoreQuery::VotingPowerAtHeight {
                                    address: who.into_string(),
                                    height: None,
                                },
                            )?;
                        if !voting_power.power.is_zero() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        // all other cases are not allowed
        Err(PreProposeError::SubmissionPolicy(
            PreProposeSubmissionPolicyError::Unauthorized {},
        ))
    }

    pub fn query(&self, deps: Deps, _env: Env, msg: QueryMsg<QueryExt>) -> StdResult<Binary> {
        match msg {
            QueryMsg::ProposalModule {} => {
                to_json_binary(&self.proposal_module.load(deps.storage)?)
            }
            QueryMsg::Dao {} => to_json_binary(&self.dao.load(deps.storage)?),
            QueryMsg::Config {} => to_json_binary(&self.config.load(deps.storage)?),
            QueryMsg::DepositInfo { proposal_id } => {
                let (deposit_info, proposer) = self.deposits.load(deps.storage, proposal_id)?;
                to_json_binary(&DepositInfoResponse {
                    deposit_info,
                    proposer,
                })
            }
            QueryMsg::CanPropose { address } => {
                let addr = deps.api.addr_validate(&address)?;
                match self.check_can_submit(deps, addr) {
                    Ok(_) => to_json_binary(&true),
                    Err(err) => match err {
                        PreProposeError::SubmissionPolicy(
                            PreProposeSubmissionPolicyError::Unauthorized {},
                        ) => to_json_binary(&false),
                        PreProposeError::Std(err) => Err(err),
                        _ => Err(StdError::generic_err(format!(
                            "unexpected error: {:?}",
                            err
                        ))),
                    },
                }
            }
            QueryMsg::ProposalSubmittedHooks {} => {
                to_json_binary(&self.proposal_submitted_hooks.query_hooks(deps)?)
            }
            QueryMsg::QueryExtension { .. } => Ok(Binary::default()),
        }
    }
}
