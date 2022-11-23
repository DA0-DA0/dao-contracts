#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, BlockInfo, Deps, DepsMut, Empty, Env, MessageInfo, OverflowError, Reply,
    Response, StdError, StdResult, Storage, SubMsg, SubMsgResult, WasmMsg,
};
use cw_utils::Expiration;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    DelegationCountResponse, DelegationResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use crate::state::{Config, Delegation, CONFIG, DELEGATIONS, DELEGATION_COUNT, EXECUTE_CTX};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cwd-proposal-delegate";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

const DEFAULT_POLICY_IRREVOCABLE: bool = false;
const DEFAULT_POLICY_PRESERVE_ON_FAILURE: bool = false;

const REPLY_ID_EXECUTE_PROPOSAL_HOOK: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin = deps.api.addr_validate(&msg.admin)?;

    CONFIG.save(deps.storage, &Config { admin })?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Delegate {
            delegate,
            msgs,
            expiration,
            policy_irrevocable,
            policy_preserve_on_failure,
        } => {
            let policy_irrevocable = policy_irrevocable.unwrap_or(DEFAULT_POLICY_IRREVOCABLE);
            let policy_preserve_on_failure =
                policy_preserve_on_failure.unwrap_or(DEFAULT_POLICY_PRESERVE_ON_FAILURE);
            let delegate = deps.api.addr_validate(&delegate)?;
            execute_delegate(
                deps,
                env,
                info,
                Delegation {
                    delegate,
                    msgs,
                    expiration,
                    policy_irrevocable,
                    policy_preserve_on_failure,
                },
            )
        }
        ExecuteMsg::RemoveDelegation { delegation_id } => {
            execute_remove_delegation(deps, env, info, delegation_id)
        }
        ExecuteMsg::Execute { delegation_id } => execute_execute(deps, env, info, delegation_id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_ID_EXECUTE_PROPOSAL_HOOK => {
            let id = EXECUTE_CTX.load(deps.storage)?;
            let Delegation {
                policy_preserve_on_failure,
                ..
            } = DELEGATIONS.load(deps.storage, id)?;

            if let SubMsgResult::Err(_) = msg.result {
                // && with `let` expression above gives unstable Rust error
                if policy_preserve_on_failure {
                    return Ok(Response::default()
                        .add_attribute("execute_failed_but_preserved", id.to_string()));
                }
            }
            // Delete delegation in both success and error case
            DELEGATIONS.remove(deps.storage, id);
            Ok(Response::default())
        }
        _ => Err(ContractError::Std(StdError::GenericErr {
            msg: "Reply handler ID not found".into(),
        })),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DelegationCount {} => Ok(to_binary(&query_delegation_count(deps)?)?),
        QueryMsg::Delegation { delegation_id } => {
            Ok(to_binary(&query_delegation(deps, delegation_id)?)?)
        }
    }
}

// MARK: Query helpers

fn query_delegation_count(deps: Deps) -> StdResult<DelegationCountResponse> {
    let count = DELEGATION_COUNT.load(deps.storage)?;
    Ok(DelegationCountResponse { count })
}

fn query_delegation(deps: Deps, delegation_id: u64) -> StdResult<DelegationResponse> {
    let delegation = DELEGATIONS.load(deps.storage, delegation_id)?;
    Ok(delegation)
}

// MARK: Execute subroutines

fn execute_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegation: Delegation,
) -> Result<Response, ContractError> {
    let Config { admin } = CONFIG.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    assert_not_expired(&delegation.expiration, &env.block)?;

    let id = advance_delegation_count(deps.storage)?;
    DELEGATIONS.save(deps.storage, id, &delegation)?;

    Ok(Response::default().add_attribute("delegate_id", id.to_string()))
}

fn execute_remove_delegation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    delegation_id: u64,
) -> Result<Response, ContractError> {
    let Config { admin } = CONFIG.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    // If delegation is irrevocable, return Error
    let Delegation {
        policy_irrevocable, ..
    } = DELEGATIONS.load(deps.storage, delegation_id)?;
    if policy_irrevocable {
        return Err(ContractError::DelegationIrrevocable {});
    }
    // Else remove the delegation
    DELEGATIONS.remove(deps.storage, delegation_id);
    Ok(Response::default())
}

fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegation_id: u64,
) -> Result<Response, ContractError> {
    let Delegation {
        delegate,
        msgs,
        expiration,
        ..
    } = DELEGATIONS.load(deps.storage, delegation_id)?;
    if delegate != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    assert_not_expired(&expiration, &env.block)?;

    let Config { admin } = CONFIG.load(deps.storage)?;
    let wasm_msg = WasmMsg::Execute {
        contract_addr: admin.to_string(),
        msg: to_binary(&cwd_core::msg::ExecuteMsg::ExecuteProposalHook { msgs })?,
        funds: vec![],
    };
    let submsg: SubMsg<Empty> = SubMsg::reply_always(wasm_msg, REPLY_ID_EXECUTE_PROPOSAL_HOOK);
    // For reply handler
    EXECUTE_CTX.save(deps.storage, &delegation_id)?;

    Ok(Response::default().add_submessage(submsg))
}

// MARK: Helpers

fn advance_delegation_count(store: &mut dyn Storage) -> StdResult<u64> {
    let lhs = DELEGATION_COUNT.may_load(store)?.unwrap_or_default();
    let res = lhs.checked_add(1);
    match res {
        Some(id) => {
            DELEGATION_COUNT.save(store, &id)?;
            Ok(id)
        }
        None => Err(StdError::Overflow {
            source: OverflowError {
                operation: cosmwasm_std::OverflowOperation::Add,
                operand1: lhs.to_string(),
                operand2: 1.to_string(),
            },
        }),
    }
}

fn assert_not_expired(
    expiration: &Option<Expiration>,
    block: &BlockInfo,
) -> Result<(), ContractError> {
    match expiration {
        Some(e) => {
            if e.is_expired(block) {
                Err(ContractError::DelegationExpired {})
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {}
