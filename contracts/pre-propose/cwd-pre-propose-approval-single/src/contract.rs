use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response,
    StdResult, WasmMsg,
};
use cw2::set_contract_version;
use cwd_interface::voting::{Query as CwCoreQuery, VotingPowerAtHeightResponse};
use cwd_pre_propose_base::{
    error::PreProposeError,
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use cwd_proposal_single::msg::ExecuteMsg as ProposalSingleExecuteMsg;
use cwd_voting::deposit::DepositRefundPolicy;

use crate::state::{APPROVER, CURRENT_ID, PENDING_PROPOSALS};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cwd-pre-propose-approval-single";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub enum ProposeMessage {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
    },
}

#[cw_serde]
pub struct InstantiateExt {
    pub approver: String,
}

#[cw_serde]
pub enum ExecuteExt {
    /// Approve a proposal, only callable by approver
    Approve { id: u64 },
    /// Reject a proposal, only callable by approver
    Reject { id: u64 },
}

#[cw_serde]
pub enum QueryExt {
    /// List the approver address
    Approver {},
    /// List of proposals awaiting approval
    Proposals {},
}

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, ExecuteExt>;
pub type QueryMsg = QueryBase<QueryExt>;

/// Internal version of the propose message that includes the
/// `proposer` field. The module will fill this in based on the sender
/// of the external message.
#[cw_serde]
pub struct ProposeMessageInternal {
    title: String,
    description: String,
    msgs: Vec<CosmosMsg<Empty>>,
    proposer: Option<String>,
}

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
    // TODO do with less code duplication from pre-propose-base?
    // Check that sender can propose
    let config = PrePropose::default().config.load(deps.storage)?;
    if !config.open_proposal_submission {
        let dao = PrePropose::default().dao.load(deps.storage)?;
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

    // Load current id
    let id = CURRENT_ID.load(deps.storage)?;

    // Save the proposal as pending
    // TODO this match is awkward?
    match msg {
        ProposeMessage::Propose {
            title,
            description,
            msgs,
        } => PENDING_PROPOSALS.save(
            deps.storage,
            id,
            &ProposeMessageInternal {
                title,
                description,
                msgs,
                proposer: Some(info.sender.to_string()),
            },
        )?,
    }

    // Save info about deposits when this prop was created
    PrePropose::default().deposits.save(
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

    // Increment next id
    CURRENT_ID.save(deps.storage, &(id + 1))?;

    // // TODO Prepare hooks msg to notify approver
    // // TODO Figure out what should be in this message
    // let hooks_msgs = PrePropose::default()
    //     .proposal_submitted_hooks
    //     .prepare_hooks(deps.storage, |a| {
    //         let execute = WasmMsg::Execute {
    //             contract_addr: a.into_string(),
    //             msg: to_binary(&msg)?,
    //             funds: vec![],
    //         };
    //         Ok(SubMsg::new(execute))
    //     })?;

    Ok(Response::default()
        .add_messages(deposit_messages)
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

    // Check proposal id exists and load proposal
    let proposal = PENDING_PROPOSALS.load(deps.storage, id)?;

    let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;
    let propose_messsage = WasmMsg::Execute {
        contract_addr: proposal_module.into_string(),
        msg: to_binary(&ProposalSingleExecuteMsg::Propose {
            title: proposal.title,
            description: proposal.description,
            msgs: proposal.msgs,
            proposer: proposal.proposer,
        })?,
        funds: vec![],
    };

    // Remove proposal
    PENDING_PROPOSALS.remove(deps.storage, id);

    Ok(Response::default().add_message(propose_messsage))
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
    PENDING_PROPOSALS.load(deps.storage, id)?;

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

pub fn execute_add_approver_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Validate address
    let addr = deps.api.addr_validate(&address)?;

    // Add hook
    PrePropose::default()
        .proposal_submitted_hooks
        .add_hook(deps.storage, addr)?;

    Ok(Response::default())
}

pub fn execute_remove_approver_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Validate address
    let addr = deps.api.addr_validate(&address)?;

    // Add hook
    PrePropose::default()
        .proposal_submitted_hooks
        .remove_hook(deps.storage, addr)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryExtension { msg } => match msg {
            QueryExt::Approver {} => to_binary(&APPROVER.load(deps.storage)?),
            QueryExt::Proposals {} => query_pending_proposals(deps, msg),
        },
        _ => PrePropose::default().query(deps, env, msg),
    }
}

// TODO potentially add other options to pending props query
pub fn query_pending_proposals(deps: Deps, _msg: QueryExt) -> StdResult<Binary> {
    let pending_proposals = PENDING_PROPOSALS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(u64, ProposeMessageInternal)>>>()?;
    to_binary(&pending_proposals)
}
