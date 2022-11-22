#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, OverflowError, Response, StdError, StdResult, Storage,
};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, Delegation, CONFIG, DELEGATION_COUNT};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cwd-proposal-delegate";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

// MARK: Execute subroutines

fn execute_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegation: Delegation,
) -> Result<Response, ContractError> {
    unimplemented!()
}

fn execute_remove_delegation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegation_id: u64,
) -> Result<Response, ContractError> {
    unimplemented!()
}

fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegation_id: u64,
) -> Result<Response, ContractError> {
    unimplemented!()
}

// MARK: Helpers

fn advance_delegate_id(store: &mut dyn Storage) -> StdResult<u64> {
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

#[cfg(test)]
mod tests {}
