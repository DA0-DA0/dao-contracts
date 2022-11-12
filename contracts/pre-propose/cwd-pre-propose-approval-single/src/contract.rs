use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cwd_pre_propose_base::{
    error::PreProposeError,
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};

use crate::state::APPROVER;

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
pub enum ProposeMessageInternal {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
        proposer: Option<String>,
    },
}

type PrePropose = PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposeMessage>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, PreProposeError> {
    let approver = deps.api.addr_validate(&msg.extension.approver)?;
    APPROVER.save(deps.storage, &approver)?;

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
            ExecuteExt::Approve { id } => execute_approve(deps, env, info, id),
            ExecuteExt::Reject { id } => execute_reject(deps, env, info, id),
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
    /*
     * Should
     * - Store prop with ID and original proposer
     * - Fire off hook? (if registered... we don't know where to send it...)
     */

    // Can sender propose?

    // Handle deposit logic?

    Ok(Response::default())
}

pub fn execute_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Check proposal id exists

    // TODO submit proposal
    // ExecuteInternal::Propose {
    //         msg: ProposeMessageInternal::Propose {
    //             // Fill in proposer based on message sender.
    //             proposer: Some(info.sender.to_string()),
    //             title,
    //             description,
    //             msgs,
    //         },
    //     },

    // Handle deposit logic?

    Ok(Response::default())
}

pub fn execute_reject(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, PreProposeError> {
    // Check sender is the approver
    let approver = APPROVER.load(deps.storage)?;
    if approver != info.sender {
        return Err(PreProposeError::Unauthorized {});
    }

    // Check proposal id exists

    // Handle deposit logic?

    Ok(Response::default())
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
            QueryExt::Approver {} => unimplemented!(),
            QueryExt::Proposals {} => unimplemented!(),
        },
        _ => PrePropose::default().query(deps, env, msg),
    }
}
