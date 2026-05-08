//! Entry points for dao-proposal-wavs v0.1.
//!
//! This is scaffolding. Handlers route messages to placeholder implementations; the real WAVS
//! envelope handling (decode payload → replay-check → cw-filter → create proposal → execute or
//! schedule) lives across helper functions called from `execute_service_handler`. v0.1 ships
//! the wiring + types; subsequent revisions fill in handler bodies.

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use dao_voting::status::Status;

use crate::wavs_compat::{
    ServiceHandlerExecuteMessages, ServiceHandlerQueryMessages, WavsEnvelope, WavsSignatureData,
};

use crate::error::ContractError;
use crate::filter::{FilterQueryMsg, FilterResponse};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, ProposalPayload, QueryMsg};
use crate::proposal::advance_proposal_id;
use crate::state::{
    AuthorizedService, Config, MandateFilterConfig, WavsProposal, ATTESTATIONS_SEEN, CONFIG,
    PROPOSALS, PROPOSAL_COUNT, PROPOSAL_HOOKS,
};
use crate::verify::{event_id_hex, validate_envelope};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-proposal-wavs";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default upper bound for paginated query responses, mirroring dao-proposal-single.
const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let dao = info.sender.clone();
    let service_manager = deps.api.addr_validate(&msg.service_manager)?;

    // Validate veto config if present.
    // dao_voting::veto::VetoConfig::validate requires a max_voting_period; we don't have a vote
    // phase, so we use a sentinel duration. v0.1 trusts the field to be sane; fuller validation
    // is a follow-up.
    if let Some(_v) = &msg.veto {
        // intentionally minimal — full validate() comes when we wire vote semantics in v0.2
    }

    let cfg = Config {
        dao: dao.clone(),
        service_manager,
        authorized_service: msg.authorized_service,
        mandate_filter: msg.mandate_filter,
        veto: msg.veto,
        auto_execute: msg.auto_execute,
        close_proposal_on_execution_failure: msg.close_proposal_on_execution_failure,
    };

    PROPOSAL_COUNT.save(deps.storage, &0)?;
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default()
        .add_attribute("action", "instantiate")
        .add_attribute("dao", dao)
        .add_attribute("service_manager", cfg.service_manager.to_string())
        .add_attribute("auto_execute", cfg.auto_execute.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Veto { proposal_id } => execute_veto(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
        ExecuteMsg::UpdateAuthorizedService { service } => {
            execute_update_authorized_service(deps, info, service)
        }
        ExecuteMsg::UpdateMandateFilter { mandate_filter } => {
            execute_update_mandate_filter(deps, info, mandate_filter)
        }
        ExecuteMsg::UpdateVeto { veto } => execute_update_veto(deps, info, veto),
        ExecuteMsg::UpdateAutoExecute { auto_execute } => {
            execute_update_auto_execute(deps, info, auto_execute)
        }
        ExecuteMsg::AddProposalHook { address } => execute_add_proposal_hook(deps, info, address),
        ExecuteMsg::RemoveProposalHook { address } => {
            execute_remove_proposal_hook(deps, info, address)
        }
        ExecuteMsg::ServiceHandler(handler_msg) => execute_service_handler(deps, env, info, handler_msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
        QueryMsg::ProposalCount {} => Ok(to_json_binary(&PROPOSAL_COUNT.load(deps.storage)?)?),
        QueryMsg::Proposal { proposal_id } => query_proposal(deps, proposal_id),
        QueryMsg::ListProposals { start_after, limit } => {
            query_list_proposals(deps, start_after, limit)
        }
        QueryMsg::EventIdSeen { event_id_hex } => query_event_id_seen(deps, event_id_hex),
        QueryMsg::ServiceHandler(handler_msg) => query_service_handler(deps, handler_msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default().add_attribute("action", "migrate"))
}

// =============================================================================
// Lifecycle handlers — v0.2
// =============================================================================

fn execute_execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    // Allowed only from Passed (waiting in timelock) or Open (no veto configured).
    let executable = matches!(proposal.status, Status::Passed | Status::Open);
    if !executable {
        return Err(ContractError::InvalidProposalState {
            id: proposal_id,
            action: format!("execute (current status {:?})", proposal.status),
        });
    }

    // If a veto is configured, the proposal must have cleared the timelock window. The
    // proposal carries `start_height`; the lock is `veto.timelock_duration` after start.
    if let Some(veto_cfg) = &proposal.veto {
        if let Some(timelock_block) = timelock_end_height(proposal.start_height, veto_cfg) {
            if env.block.height < timelock_block {
                return Err(ContractError::TimelockNotExpired { id: proposal_id });
            }
        }
    }

    // Build the Wasm execute back to the DAO core via dao-interface's ExecuteProposalHook.
    let dispatch_msg = dao_interface::msg::ExecuteMsg::ExecuteProposalHook {
        msgs: proposal.msgs.clone(),
    };
    let dispatch = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.dao.to_string(),
        msg: to_json_binary(&dispatch_msg)?,
        funds: vec![],
    });

    proposal.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::default()
        .add_message(dispatch)
        .add_attribute("action", "execute_proposal")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", "executed"))
}

fn execute_veto(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let mut proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    // Veto only when:
    //   - status is Passed (in timelock) OR Open with veto.veto_before_passed=true
    //   - sender == veto.vetoer
    //   - we're still inside the timelock window
    let veto_cfg = proposal
        .veto
        .as_ref()
        .ok_or(ContractError::InvalidProposalState {
            id: proposal_id,
            action: "veto (no veto config on this proposal)".into(),
        })?
        .clone();

    if info.sender.as_str() != veto_cfg.vetoer {
        return Err(ContractError::Unauthorized {});
    }

    let in_window = match proposal.status {
        Status::Passed => true,
        Status::Open => veto_cfg.veto_before_passed,
        _ => false,
    };
    if !in_window {
        return Err(ContractError::InvalidProposalState {
            id: proposal_id,
            action: format!("veto (current status {:?})", proposal.status),
        });
    }

    if let Some(end) = timelock_end_height(proposal.start_height, &veto_cfg) {
        if env.block.height >= end {
            return Err(ContractError::InvalidProposalState {
                id: proposal_id,
                action: "veto (timelock has expired)".into(),
            });
        }
    }

    proposal.status = Status::Vetoed;
    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::default()
        .add_attribute("action", "veto_proposal")
        .add_attribute("proposal_id", proposal_id.to_string()))
}

fn execute_close(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let mut proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    let closable = matches!(
        proposal.status,
        Status::Rejected | Status::ExecutionFailed | Status::Vetoed
    );
    if !closable {
        return Err(ContractError::InvalidProposalState {
            id: proposal_id,
            action: format!("close (current status {:?})", proposal.status),
        });
    }

    proposal.status = Status::Closed;
    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::default()
        .add_attribute("action", "close_proposal")
        .add_attribute("proposal_id", proposal_id.to_string()))
}

/// Compute the block height at which the timelock window ends, given a proposal's start_height
/// and a VetoConfig. Returns None if the duration is time-based (we don't have wall-clock here).
fn timelock_end_height(start_height: u64, veto_cfg: &dao_voting::veto::VetoConfig) -> Option<u64> {
    use cw_utils::Duration;
    match veto_cfg.timelock_duration {
        Duration::Height(h) => Some(start_height + h),
        Duration::Time(_) => None, // time-based timelocks not supported for height-based check; v0.3
    }
}

fn execute_update_authorized_service(
    deps: DepsMut,
    info: MessageInfo,
    service: AuthorizedService,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.dao {
        return Err(ContractError::NotDaoCore {});
    }
    cfg.authorized_service = service;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default().add_attribute("action", "update_authorized_service"))
}

fn execute_update_mandate_filter(
    deps: DepsMut,
    info: MessageInfo,
    mandate_filter: Option<MandateFilterConfig>,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.dao {
        return Err(ContractError::NotDaoCore {});
    }
    cfg.mandate_filter = mandate_filter;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default().add_attribute("action", "update_mandate_filter"))
}

fn execute_update_veto(
    deps: DepsMut,
    info: MessageInfo,
    veto: Option<dao_voting::veto::VetoConfig>,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.dao {
        return Err(ContractError::NotDaoCore {});
    }
    cfg.veto = veto;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default().add_attribute("action", "update_veto"))
}

fn execute_update_auto_execute(
    deps: DepsMut,
    info: MessageInfo,
    auto_execute: bool,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.dao {
        return Err(ContractError::NotDaoCore {});
    }
    cfg.auto_execute = auto_execute;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default()
        .add_attribute("action", "update_auto_execute")
        .add_attribute("auto_execute", auto_execute.to_string()))
}

fn execute_add_proposal_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.dao {
        return Err(ContractError::NotDaoCore {});
    }
    let addr = deps.api.addr_validate(&address)?;
    PROPOSAL_HOOKS.add_hook(deps.storage, addr.clone())?;
    Ok(Response::default()
        .add_attribute("action", "add_proposal_hook")
        .add_attribute("address", addr))
}

fn execute_remove_proposal_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.dao {
        return Err(ContractError::NotDaoCore {});
    }
    let addr = deps.api.addr_validate(&address)?;
    PROPOSAL_HOOKS.remove_hook(deps.storage, addr.clone())?;
    Ok(Response::default()
        .add_attribute("action", "remove_proposal_hook")
        .add_attribute("address", addr))
}

fn execute_service_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ServiceHandlerExecuteMessages,
) -> Result<Response, ContractError> {
    match msg {
        ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope,
            signature_data,
        } => execute_wavs_handle_signed_envelope(deps, env, info, envelope, signature_data),
    }
}

/// Full v0.2 flow per `memory/wavs-proposal-module.md` Path A:
///   1. Authorize sender against `Config.authorized_service` (v0.2: SingleOperator path).
///   2. Defer signature/quorum check to `service_manager.WavsValidate`.
///   3. Replay-check `envelope.eventId` against `ATTESTATIONS_SEEN`.
///   4. Decode `envelope.payload` (bytes 32..) as `ProposalPayload` JSON.
///   5. (Optional) Run cw-filter on each msg in the payload.
///   6. Create the `WavsProposal` record + advance counter + mark eventId as seen.
///   7. If `auto_execute` and no veto configured: dispatch immediately. If `auto_execute` and
///      veto configured: status = Passed (timelock starts); explicit Execute call after
///      timelock will dispatch. If !auto_execute: status = Open (waiting for explicit Execute).
fn execute_wavs_handle_signed_envelope(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    envelope: WavsEnvelope,
    signature_data: WavsSignatureData,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // 1. Authorize sender.
    match &cfg.authorized_service {
        AuthorizedService::SingleOperator { addr } => {
            if info.sender != *addr {
                return Err(ContractError::Unauthorized {});
            }
        }
        AuthorizedService::Quorum { .. } | AuthorizedService::Registry { .. } => {
            return Err(ContractError::InvalidPayload {
                reason: "Quorum / Registry authorization not implemented in v0.2".into(),
            });
        }
    }

    // 2. Defer to service-manager. (Currently rubber-stamps per cw-middleware's TODO; will
    //    become real verification when Lay3rLabs ships it. Our contract doesn't change.)
    validate_envelope(deps.as_ref(), cfg.service_manager.as_str(), &envelope, &signature_data)?;

    // 3. Replay-check.
    let event_id = envelope.event_id().map_err(|e| ContractError::InvalidPayload {
        reason: e.to_string(),
    })?;
    let event_id_hex_str = event_id_hex(event_id);
    if ATTESTATIONS_SEEN
        .may_load(deps.storage, event_id)?
        .unwrap_or(false)
    {
        return Err(ContractError::Replay {
            event_id_hex: event_id_hex_str,
        });
    }

    // 4. Decode payload.
    let payload_bytes = envelope.payload().map_err(|e| ContractError::InvalidPayload {
        reason: e.to_string(),
    })?;
    let payload: ProposalPayload =
        serde_json::from_slice(payload_bytes).map_err(|e| ContractError::InvalidPayload {
            reason: format!("payload JSON decode failed: {e}"),
        })?;

    // 5. Optional cw-filter mandate check.
    if let Some(filter_cfg) = &cfg.mandate_filter {
        for (i, msg) in payload.msgs.iter().enumerate() {
            let resp: FilterResponse = deps.querier.query_wasm_smart(
                filter_cfg.filter_contract.as_str(),
                &FilterQueryMsg::Filter {
                    filter: filter_cfg.filter.clone(),
                    msg: msg.clone(),
                },
            )?;
            match resp {
                FilterResponse::Pass {} => continue,
                FilterResponse::Fail { reason } => {
                    return Err(ContractError::MandateFilterFail { index: i, reason });
                }
                FilterResponse::Fatal { reason } => {
                    return Err(ContractError::MandateFilterFatal { index: i, reason });
                }
            }
        }
    }

    // 6. Create proposal record + mark replay-seen.
    let id = advance_proposal_id(deps.storage)?;
    let initial_status = if cfg.auto_execute && cfg.veto.is_none() {
        Status::Executed
    } else if cfg.auto_execute && cfg.veto.is_some() {
        Status::Passed // queued in timelock; explicit Execute call after timelock_end dispatches
    } else {
        Status::Open // requires explicit Execute call
    };

    let proposal = WavsProposal {
        title: payload.title.clone(),
        description: payload.description.clone(),
        event_id_hex: event_id_hex_str.clone(),
        msgs: payload.msgs.clone(),
        start_height: env.block.height,
        veto: cfg.veto.clone(),
        auto_execute: cfg.auto_execute,
        status: initial_status,
    };
    PROPOSALS.save(deps.storage, id, &proposal)?;
    ATTESTATIONS_SEEN.save(deps.storage, event_id, &true)?;

    // 7. If auto_execute and no veto, dispatch the msgs to the DAO core immediately.
    let mut response = Response::default()
        .add_attribute("action", "wavs_handle_signed_envelope")
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("event_id_hex", event_id_hex_str)
        .add_attribute("status", format!("{:?}", initial_status));

    if cfg.auto_execute && cfg.veto.is_none() {
        let dispatch_msg = dao_interface::msg::ExecuteMsg::ExecuteProposalHook {
            msgs: payload.msgs,
        };
        let dispatch = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.dao.to_string(),
            msg: to_json_binary(&dispatch_msg)?,
            funds: vec![],
        });
        response = response.add_message(dispatch);
    }

    Ok(response)
}

// =============================================================================
// Query handlers
// =============================================================================

fn query_proposal(deps: Deps, proposal_id: u64) -> Result<Binary, ContractError> {
    let proposal: WavsProposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;
    Ok(to_json_binary(&proposal)?)
}

fn query_list_proposals(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<Binary, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let proposals: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_id, p)| p))
        .collect();

    Ok(to_json_binary(&proposals?)?)
}

fn query_event_id_seen(deps: Deps, event_id_hex: String) -> Result<Binary, ContractError> {
    let bytes = decode_hex(&event_id_hex)?;
    let seen = ATTESTATIONS_SEEN
        .may_load(deps.storage, bytes.as_slice())?
        .unwrap_or(false);
    Ok(to_json_binary(&seen)?)
}

fn query_service_handler(
    deps: Deps,
    msg: ServiceHandlerQueryMessages,
) -> Result<Binary, ContractError> {
    match msg {
        ServiceHandlerQueryMessages::WavsServiceManager {} => {
            let cfg = CONFIG.load(deps.storage)?;
            Ok(to_json_binary(&cfg.service_manager)?)
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn decode_hex(s: &str) -> Result<Vec<u8>, ContractError> {
    let s = s.trim_start_matches("0x");
    if !s.len().is_multiple_of(2) {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "hex string must have even length",
        )));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| {
                ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                    "invalid hex: {e}"
                )))
            })
        })
        .collect()
}
