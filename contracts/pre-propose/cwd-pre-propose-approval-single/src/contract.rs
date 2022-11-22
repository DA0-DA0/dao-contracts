#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult,
    Storage, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_paginate::paginate_map_values;
use cwd_pre_propose_base::{
    error::PreProposeError, msg::ExecuteMsg as ExecuteBase, state::PreProposeContract,
};
use cwd_proposal_single::msg::ProposeMsg;
use cwd_voting::deposit::DepositRefundPolicy;

use crate::msg::{
    ApproverProposeMessage, ExecuteExt, ExecuteMsg, InstantiateExt, InstantiateMsg, ProposeMessage,
    ProposeMessageInternal, QueryExt, QueryMsg,
};
use crate::state::{PendingProposal, APPROVER, CURRENT_ID, PENDING_PROPOSALS};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cwd-pre-propose-approval-single";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

type PrePropose = PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposeMessage>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, PreProposeError> {
    // Validate and save approver address
    let approver = deps.api.addr_validate(&msg.extension.approver)?;
    APPROVER.save(deps.storage, &approver)?;

    // Initialize first proposal ID
    CURRENT_ID.save(deps.storage, &0)?;

    let resp = PrePropose::default().instantiate(deps.branch(), env, info, msg)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(resp.add_attribute("approver", approver.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, PreProposeError> {
    match msg {
        // Override pre-propose-base behavior
        ExecuteMsg::Propose { msg } => execute_propose(deps, env, info, msg),
        ExecuteMsg::AddProposalSubmittedHook { address } => {
            execute_add_approver_hook(deps, info, address)
        }
        ExecuteMsg::RemoveProposalSubmittedHook { address } => {
            execute_remove_approver_hook(deps, info, address)
        }
        // Extension
        ExecuteMsg::Extension { msg } => match msg {
            ExecuteExt::Approve { id } => execute_approve(deps, info, id),
            ExecuteExt::Reject { id } => execute_reject(deps, info, id),
            ExecuteExt::UpdateApprover { address } => execute_update_approver(deps, info, address),
        },
        // Default pre-propose-base behavior for all other messages
        _ => PrePropose::default().execute(deps, env, info, msg),
    }
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ProposeMessage,
) -> Result<Response, PreProposeError> {
    // Base pre-propose contract with our configured exetensions
    let pre_propose_base = PrePropose::default();
    let config = pre_propose_base.config.load(deps.storage)?;

    // Check that sender can propose
    pre_propose_base.check_can_submit(deps.as_ref(), info.clone())?;

    // Load current id
    let id = advance_proposal_id(deps.storage)?;

    // Convert msg to to internal format
    let propose_msg_internal = match msg {
        ProposeMessage::Propose {
            title,
            description,
            msgs,
        } => ProposeMsg {
            title,
            description,
            msgs,
            proposer: Some(info.sender.to_string()),
        },
    };

    // Save the proposal as pending
    PENDING_PROPOSALS.save(
        deps.storage,
        id,
        &PendingProposal {
            id,
            msg: propose_msg_internal.clone(),
        },
    )?;

    // Save info about deposits when this prop was created
    pre_propose_base.deposits.save(
        deps.storage,
        id,
        &(config.deposit_info.clone(), info.sender.clone()),
    )?;

    // Handle deposit if configured
    let deposit_messages = if let Some(ref deposit_info) = config.deposit_info {
        deposit_info.check_native_deposit_paid(&info)?;
        deposit_info.get_take_deposit_messages(&info.sender, &env.contract.address)?
    } else {
        vec![]
    };

    // Prepare proposal submitted hooks msg to notify approver
    // Make a proposal on the approver DAO to approve this pre-proposal
    let hooks_msgs =
        pre_propose_base
            .proposal_submitted_hooks
            .prepare_hooks(deps.storage, |a| {
                let execute_msg = WasmMsg::Execute {
                    contract_addr: a.into_string(),
                    msg: to_binary(&ExecuteBase::<ApproverProposeMessage, Empty>::Propose {
                        msg: ApproverProposeMessage::Propose {
                            title: propose_msg_internal.clone().title,
                            description: propose_msg_internal.clone().description,
                            pre_propose_id: id,
                        },
                    })?,
                    funds: vec![],
                };
                Ok(SubMsg::new(execute_msg))
            })?;

    Ok(Response::default()
        .add_messages(deposit_messages)
        .add_submessages(hooks_msgs)
        .add_attribute("method", "pre-propose")
        .add_attribute("id", id.to_string()))
}

pub fn execute_approve(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Load proposal and send propose message to the proposal module
    let proposal = PENDING_PROPOSALS.may_load(deps.storage, id)?;
    match proposal {
        Some(proposal) => {
            let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;
            let propose_messsage = WasmMsg::Execute {
                contract_addr: proposal_module.into_string(),
                msg: to_binary(&ProposeMessageInternal::Propose(ProposeMsg {
                    title: proposal.msg.title,
                    description: proposal.msg.description,
                    msgs: proposal.msg.msgs,
                    proposer: proposal.msg.proposer,
                }))?,
                funds: vec![],
            };

            // Remove proposal
            PENDING_PROPOSALS.remove(deps.storage, id);

            Ok(Response::default()
                .add_message(propose_messsage)
                .add_attribute("method", "proposal_approved")
                .add_attribute("proposal", id.to_string()))
        }
        None => Err(PreProposeError::ProposalNotFound {}),
    }
}

pub fn execute_reject(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Check proposal id exists
    ensure!(
        PENDING_PROPOSALS.has(deps.storage, id),
        PreProposeError::ProposalNotFound {}
    );

    // Remove proposal
    PENDING_PROPOSALS.remove(deps.storage, id);

    // Handle deposit logic
    match PrePropose::default().deposits.may_load(deps.storage, id)? {
        Some((deposit_info, proposer)) => {
            let messages = if let Some(ref deposit_info) = deposit_info {
                // Refund can be issued if proposal if deposits are always refunded
                // OnlyPassed and Never refund deposit policies do not apply here
                if deposit_info.refund_policy == DepositRefundPolicy::Always {
                    deposit_info.get_return_deposit_message(&proposer)?
                } else {
                    // If the proposer doesn't get the deposit, the DAO does.
                    let dao = PrePropose::default().dao.load(deps.storage)?;
                    deposit_info.get_return_deposit_message(&dao)?
                }
            } else {
                // No deposit info for this proposal. Nothing to do.
                vec![]
            };

            Ok(Response::default()
                .add_attribute("method", "proposal_rejected")
                .add_attribute("proposal", id.to_string())
                .add_attribute("deposit_info", to_binary(&deposit_info)?.to_string())
                .add_messages(messages))
        }

        // If we do not have a deposit for this proposal it was
        // likely created before we were added to the proposal
        // module. In that case, it's not our problem and we just
        // do nothing.
        None => Ok(Response::default()
            .add_attribute("method", "proposal_rejected")
            .add_attribute("proposal", id.to_string())),
    }
}

pub fn execute_update_approver(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Validate address and save new approver
    let addr = deps.api.addr_validate(&address)?;
    APPROVER.save(deps.storage, &addr)?;

    Ok(Response::default())
}

pub fn execute_add_approver_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, PreProposeError> {
    let pre_propose_base = PrePropose::default();

    let dao = pre_propose_base.dao.load(deps.storage)?;
    let approver = APPROVER.load(deps.storage)?;

    // Check sender is the approver or the parent DAO
    if approver != info.sender && dao != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Validate address
    let addr = deps.api.addr_validate(&address)?;

    // Add hook
    pre_propose_base
        .proposal_submitted_hooks
        .add_hook(deps.storage, addr)?;

    Ok(Response::default())
}

pub fn execute_remove_approver_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, PreProposeError> {
    let pre_propose_base = PrePropose::default();

    let dao = pre_propose_base.dao.load(deps.storage)?;
    let approver = APPROVER.load(deps.storage)?;

    // Check sender is the approver or the parent DAO
    if approver != info.sender && dao != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Validate address
    let addr = deps.api.addr_validate(&address)?;

    // Add hook
    pre_propose_base
        .proposal_submitted_hooks
        .remove_hook(deps.storage, addr)?;

    Ok(Response::default())
}

pub fn advance_proposal_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = CURRENT_ID.may_load(store)?.unwrap_or_default() + 1;
    CURRENT_ID.save(store, &id)?;
    Ok(id)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryExtension { msg } => match msg {
            QueryExt::Approver {} => to_binary(&APPROVER.load(deps.storage)?),
            QueryExt::PendingProposal { id } => {
                to_binary(&PENDING_PROPOSALS.load(deps.storage, id)?)
            }
            QueryExt::PendingProposals { start_after, limit } => to_binary(&paginate_map_values(
                deps,
                &PENDING_PROPOSALS,
                start_after,
                limit,
                Order::Descending,
            )?),
            QueryExt::ReversePendingProposals { start_after, limit } => {
                to_binary(&paginate_map_values(
                    deps,
                    &PENDING_PROPOSALS,
                    start_after,
                    limit,
                    Order::Ascending,
                )?)
            }
        },
        _ => PrePropose::default().query(deps, env, msg),
    }
}
