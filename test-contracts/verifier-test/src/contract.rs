use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InnerExecuteMsg, InstantiateMsg, QueryMsg},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw_verifier_middleware::verify::verify;

const CONTRACT_NAME: &str = "crates.io:cw-verifier-test";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let (verified_msg, verified_info) =
        verify::<InnerExecuteMsg>(deps.api, deps.storage, &env, info, msg.wrapped_msg)?;
    match verified_msg {
        InnerExecuteMsg::Execute => execute_execute(deps, env, verified_info)?,
    };
    Ok(Response::default())
}

pub fn execute_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    Ok(Response::default().add_attribute("action", "execute_execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Ok(Binary::default())
}
