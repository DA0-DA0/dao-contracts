use cosmwasm_std::{
    to_binary, Binary, ContractInfoResponse, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;

use cw_core_interface::voting::{Query as CwCoreQuery, VotingPowerAtHeightResponse};
use cw_denom::UncheckedDenom;
use proposal_hooks::ProposalHookMsg;
use serde::Serialize;
use voting::deposit::{DepositRefundPolicy, UncheckedDepositInfo};

use crate::{
    error::PreProposeError,
    msg::{DepositInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{Config, PreProposeContract},
};

const CONTRACT_NAME: &str = "crates.io::cw-dao-pre-propose-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
    PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
where
    ProposalMessage: Serialize,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: InstantiateMsg<InstantiateExt>,
    ) -> Result<Response, PreProposeError> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // The proposal module instantiates us. Same as below, we're
        // making limited assumptions here. The only way to associate
        // a deposit module with a proposal module is for the proposal
        // module to instantiate it.
        self.proposal_module.save(deps.storage, &info.sender)?;

        // The DAO instantiates the proposal module. Note that we're
        // making limited assumptions here. The only way to add a
        // proposal module to a DAO is for it to create it.
        let module_info: ContractInfoResponse =
            deps.querier
                .query(&QueryRequest::Wasm(WasmQuery::ContractInfo {
                    contract_addr: info.sender.to_string(),
                }))?;
        let dao = deps.api.addr_validate(&module_info.creator)?;
        self.dao.save(deps.storage, &dao)?;

        let deposit_info = msg
            .deposit_info
            .map(|info| info.into_checked(deps.as_ref(), dao))
            .transpose()?;

        let config = Config {
            deposit_info,
            open_proposal_submission: msg.open_proposal_submission,
        };
        self.config.save(deps.storage, &config)?;

        Ok(Response::default()
            .add_attribute("method", "instantiate")
            .add_attribute("proposal_module", info.sender.into_string())
            .add_attribute("deposit_info", format!("{:?}", config.deposit_info))
            .add_attribute(
                "open_proposal_submission",
                config.open_proposal_submission.to_string(),
            )
            .add_attribute("dao", module_info.creator))
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<ProposalMessage, ExecuteExt>,
    ) -> Result<Response, PreProposeError> {
        match msg {
            ExecuteMsg::Propose { msg } => self.execute_propose(deps.as_ref(), env, info, msg),
            ExecuteMsg::UpdateConfig {
                deposit_info,
                open_proposal_submission,
            } => self.execute_update_config(deps, info, deposit_info, open_proposal_submission),
            ExecuteMsg::Withdraw { denom } => {
                self.execute_withdraw(deps.as_ref(), env, info, denom)
            }

            ExecuteMsg::Extension { .. } => Ok(Response::default()),

            ExecuteMsg::ProposalHook(ProposalHookMsg::ProposalStatusChanged {
                id,
                new_status,
                // Deposits only care about where you're headed, not
                // where you've been.
                old_status: _,
            }) => self.execute_status_change_proposal_hook(deps.as_ref(), info, id, new_status),
            ExecuteMsg::ProposalHook(ProposalHookMsg::NewProposal { id, proposer }) => {
                self.execute_new_proposal_hook(deps, info, id, proposer)
            }
        }
    }

    pub fn query(&self, deps: Deps, _env: Env, msg: QueryMsg<QueryExt>) -> StdResult<Binary> {
        match msg {
            QueryMsg::ProposalModule {} => to_binary(&self.proposal_module.load(deps.storage)?),
            QueryMsg::Dao {} => to_binary(&self.dao.load(deps.storage)?),
            QueryMsg::Config {} => to_binary(&self.config.load(deps.storage)?),
            QueryMsg::DepositInfo { proposal_id } => {
                let (deposit_info, proposer) = self.deposits.load(deps.storage, proposal_id)?;
                to_binary(&DepositInfoResponse {
                    deposit_info,
                    proposer,
                })
            }
            QueryMsg::Extension { .. } => Ok(Binary::default()),
        }
    }

    pub fn execute_propose(
        &self,
        deps: Deps,
        env: Env,
        info: MessageInfo,
        msg: ProposalMessage,
    ) -> Result<Response, PreProposeError> {
        let config = self.config.load(deps.storage)?;

        if !config.open_proposal_submission {
            let dao = self.dao.load(deps.storage)?;
            let voting_power: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
                dao.into_string(),
                &CwCoreQuery::VotingPowerAtHeight {
                    address: info.sender.to_string(),
                    height: None,
                },
            )?;
            if voting_power.power.is_zero() {
                return Err(PreProposeError::NotMember {});
            }
        }

        let deposit_messages = if let Some(ref deposit_info) = config.deposit_info {
            deposit_info.check_native_deposit_paid(&info)?;
            deposit_info.get_take_deposit_messages(&info.sender, &env.contract.address)?
        } else {
            vec![]
        };

        let proposal_module = self.proposal_module.load(deps.storage)?;
        let propose_messsage = WasmMsg::Execute {
            contract_addr: proposal_module.into_string(),
            msg: to_binary(&msg)?,
            funds: vec![],
        };

        Ok(Response::default()
            .add_attribute("method", "execute_propose")
            .add_attribute("sender", info.sender)
            .add_attribute("deposit_info", to_binary(&config.deposit_info)?.to_string())
            .add_messages(deposit_messages)
            .add_message(propose_messsage))
    }

    pub fn execute_update_config(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        deposit_info: Option<UncheckedDepositInfo>,
        open_proposal_submission: bool,
    ) -> Result<Response, PreProposeError> {
        let dao = self.dao.load(deps.storage)?;
        if info.sender != dao {
            Err(PreProposeError::NotDao {})
        } else {
            let deposit_info = deposit_info
                .map(|d| d.into_checked(deps.as_ref(), dao))
                .transpose()?;
            self.config.save(
                deps.storage,
                &Config {
                    deposit_info,
                    open_proposal_submission,
                },
            )?;

            Ok(Response::default()
                .add_attribute("method", "update_config")
                .add_attribute("sender", info.sender))
        }
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

    pub fn execute_status_change_proposal_hook(
        &self,
        deps: Deps,
        info: MessageInfo,
        id: u64,
        new_status: String,
    ) -> Result<Response, PreProposeError> {
        let proposal_module = self.proposal_module.load(deps.storage)?;
        if info.sender != proposal_module {
            return Err(PreProposeError::NotModule {});
        }

        match self.deposits.may_load(deps.storage, id)? {
            Some((deposit_info, proposer)) => {
                let messages = if let Some(ref deposit_info) = deposit_info {
                    // If the proposal is completed, either return to the DAO
                    // or issue a refund.
                    let proposal_completed = new_status == "closed" || new_status == "executed";

                    if proposal_completed {
                        // Refund can be issued if proposal if it is going to
                        // closed or executed.
                        let should_refund_to_proposer = (new_status == "closed"
                            && deposit_info.refund_policy == DepositRefundPolicy::Always)
                            || (new_status == "executed"
                                && deposit_info.refund_policy != DepositRefundPolicy::Never);

                        if should_refund_to_proposer {
                            deposit_info.get_return_deposit_message(&proposer)?
                        } else {
                            // If the proposer doesn't get the deposit, the DAO does.
                            let dao = self.dao.load(deps.storage)?;
                            deposit_info.get_return_deposit_message(&dao)?
                        }
                    } else {
                        // Proposal isn't done. Nothing to do.
                        vec![]
                    }
                } else {
                    // No for this proposal. Nothing to do.
                    vec![]
                };

                Ok(Response::default()
                    .add_attribute("method", "execute_status_changed_proposal_hook")
                    .add_attribute("proposal", id.to_string())
                    .add_attribute("deposit_info", to_binary(&deposit_info)?.to_string())
                    .add_messages(messages))
            }
            // If we do not have a deposit for this proposal it was
            // likely created before we were added to the proposal
            // module. In that case, it's not our problem and we just
            // do nothing.
            None => Ok(Response::default()
                .add_attribute("method", "execute_status_changed_proposal_hook")
                .add_attribute("proposal", id.to_string())),
        }
    }

    pub fn execute_new_proposal_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: u64,
        proposer: String,
    ) -> Result<Response, PreProposeError> {
        let proposer = deps.api.addr_validate(&proposer)?;
        let proposal_module = self.proposal_module.load(deps.storage)?;
        if info.sender != proposal_module {
            return Err(PreProposeError::NotModule {});
        }

        // Save the deposit.
        //
        // FIXME(zeke): This makes me nervous. Does there exist a
        // sequence of events where the deposit info changes between
        // the new proposal hook being received and us dispatching the
        // message to create a proposal?
        let config = self.config.load(deps.storage)?;
        self.deposits
            .save(deps.storage, id, &(config.deposit_info, proposer))?;

        Ok(Response::default()
            .add_attribute("method", "execute_new_proposal_hook")
            .add_attribute("proposal_id", id.to_string()))
    }
}
