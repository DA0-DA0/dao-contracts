use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsg, SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use cwd_interface::ModuleInstantiateCallback;
use cwd_pre_propose_approval_single::{
    ApproverProposeMessage, ExecuteExt as ApprovalExt, ExecuteMsg as PreProposeApprovalExecuteMsg,
};
use cwd_pre_propose_base::{
    error::PreProposeError,
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use cwd_proposal_single::msg::ExecuteMsg as ProposalSingleExecuteMsg;
use cwd_voting::status::Status;

pub(crate) const CONTRACT_NAME: &str = "crates.io:cwd-pre-propose-approver";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct InstantiateMsg {
    pub pre_propose_approval_contract: String,
}

#[cw_serde]
pub enum QueryExt {
    PreProposeApprovalContract {},
}

pub type BaseInstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ApproverProposeMessage, Empty>;
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

// Stores the address of the pre-propose approval contract
pub const PRE_PROPOSE_APPROVAL_CONTRACT: Item<Addr> = Item::new("pre_propose_approval_contract");
// Stores the current pre-propose-id for use in submessage reply
pub const CURRENT_PRE_PROPOSE_ID: Item<u64> = Item::new("current_pre_propose_id");
// Maps proposal ids to pre-propose ids
pub const PROPOSAL_IDS: Map<u64, u64> = Map::new("proposal_ids");

type PrePropose = PreProposeContract<Empty, Empty, QueryExt, ApproverProposeMessage>;

const APPROVER_PROPOSE_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, PreProposeError> {
    // This contract does not handle deposits or have open submissions
    // Here we hardcode the pre-propose-base instantiate message
    let base_instantiate_msg = BaseInstantiateMsg {
        deposit_info: None,
        open_proposal_submission: false,
        extension: Empty {},
    };
    // Default pre-propose-base instantiation
    let resp = PrePropose::default().instantiate(
        deps.branch(),
        env.clone(),
        info,
        base_instantiate_msg,
    )?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate and save the address of the pre-propose-approval-single contract
    let addr = deps.api.addr_validate(&msg.pre_propose_approval_contract)?;
    PRE_PROPOSE_APPROVAL_CONTRACT.save(deps.storage, &addr)?;

    Ok(resp.set_data(to_binary(&ModuleInstantiateCallback {
        msgs: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_binary(&PreProposeApprovalExecuteMsg::AddProposalSubmittedHook {
                    address: env.contract.address.to_string(),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_binary(&PreProposeApprovalExecuteMsg::Extension {
                    msg: ApprovalExt::UpdateApprover {
                        address: env.contract.address.to_string(),
                    },
                })?,
                funds: vec![],
            }),
        ],
    })?))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, PreProposeError> {
    match msg {
        // Override default pre-propose-base behavior
        ExecuteMsg::Propose { msg } => execute_propose(deps, info, msg),
        ExecuteMsg::ProposalCompletedHook {
            proposal_id,
            new_status,
        } => execute_proposal_completed(deps, info, proposal_id, new_status),
        _ => PrePropose::default().execute(deps, env, info, msg),
    }
}

pub fn execute_propose(
    deps: DepsMut,
    info: MessageInfo,
    msg: ApproverProposeMessage,
) -> Result<Response, PreProposeError> {
    // Check that this is coming from the expected approval contract
    let approval_contract = PRE_PROPOSE_APPROVAL_CONTRACT.load(deps.storage)?;
    if info.sender != approval_contract {
        return Err(PreProposeError::Unauthorized {});
    }

    // Get pre_prospose_id, transform proposal for the approver
    // Here we make sure that there are no messages that can be executed
    let (pre_propose_id, sanitized_msg) = match msg {
        ApproverProposeMessage::Propose {
            title,
            description,
            pre_propose_id,
        } => (
            pre_propose_id,
            ProposalSingleExecuteMsg::Propose {
                title,
                description,
                msgs: vec![],
                proposer: Some(info.sender.to_string()),
            },
        ),
    };

    // Get proposal id from submessage
    let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;
    let propose_messsage = SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: proposal_module.into_string(),
            msg: to_binary(&sanitized_msg)?,
            funds: vec![],
        },
        APPROVER_PROPOSE_REPLY_ID,
    );

    // Save current pre-propose-id for use in submessage reply
    CURRENT_PRE_PROPOSE_ID.save(deps.storage, &pre_propose_id)?;

    Ok(Response::default().add_submessage(propose_messsage))
}

pub fn execute_proposal_completed(
    deps: DepsMut,
    info: MessageInfo,
    proposal_id: u64,
    new_status: Status,
) -> Result<Response, PreProposeError> {
    // Safety check, this message can only come from the proposal module
    let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;
    if info.sender != proposal_module {
        return Err(PreProposeError::NotModule {});
    }

    // Get approval pre-propose id
    let pre_propose_id = PROPOSAL_IDS.load(deps.storage, proposal_id)?;

    // Get approval contract address
    let approval_contract = PRE_PROPOSE_APPROVAL_CONTRACT.load(deps.storage)?;

    // On completion send rejection or approval message
    let msg = match new_status {
        Status::Closed => Some(WasmMsg::Execute {
            contract_addr: approval_contract.into_string(),
            msg: to_binary(&PreProposeApprovalExecuteMsg::Extension {
                msg: ApprovalExt::Reject { id: pre_propose_id },
            })?,
            funds: vec![],
        }),
        Status::Executed => Some(WasmMsg::Execute {
            contract_addr: approval_contract.into_string(),
            msg: to_binary(&PreProposeApprovalExecuteMsg::Extension {
                msg: ApprovalExt::Approve { id: pre_propose_id },
            })?,
            funds: vec![],
        }),
        _ => None,
    };

    // If Status is not Executed or Closed, throw error
    match msg {
        Some(msg) => Ok(Response::default()
            .add_message(msg)
            .add_attribute("method", "execute_proposal_completed_hook")
            .add_attribute("proposal", proposal_id.to_string())),
        None => Err(PreProposeError::NotClosedOrExecuted { status: new_status }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryExtension { msg } => match msg {
            QueryExt::PreProposeApprovalContract {} => {
                to_binary(&PRE_PROPOSE_APPROVAL_CONTRACT.load(deps.storage)?)
            }
        },
        _ => PrePropose::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, PreProposeError> {
    match msg.id {
        APPROVER_PROPOSE_REPLY_ID => {
            match msg.result {
                SubMsgResult::Ok(result) => {
                    let pre_propose_id = CURRENT_PRE_PROPOSE_ID.load(deps.storage)?;

                    // Parse SubMessageResponse to get proposal id
                    let event = &result.events[result.events.len() - 1];
                    let proposal_id = event.attributes[event.attributes.len() - 1]
                        .value
                        .trim()
                        .parse::<u64>()?;

                    // Save mapping of proposal id to pre propose id
                    PROPOSAL_IDS.save(deps.storage, proposal_id, &pre_propose_id)?;

                    Ok(Response::default())
                }
                SubMsgResult::Err(e) => Err(PreProposeError::Std(StdError::GenericErr { msg: e })),
            }
        }
        _ => Err(PreProposeError::UnknownReplyID {}),
    }
}
