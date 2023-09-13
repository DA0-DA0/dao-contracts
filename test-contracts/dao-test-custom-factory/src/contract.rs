#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = "CARGO_PKG_NAME";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_BASE_MINTER_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
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
    match msg {
        ExecuteMsg::StargazeBaseMinterFactory(msg) => {
            execute_stargaze_base_minter_factory(deps, env, info, msg)
        }
    }
}

pub fn execute_token_factory_factory(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: WasmMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

pub fn execute_stargaze_base_minter_factory(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: WasmMsg,
) -> Result<Response, ContractError> {
    // TODO query voting contract (the sender) for the DAO address
    // TODO replace the Stargaze info to set the DAO address

    // TODO call base-factory to create minter

    // TODO this create a base-minter contract and a sg721 contract

    // in submsg reply, parse the response and save the contract address
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => query_info(deps),
    }
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&dao_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_BASE_MINTER_REPLY_ID => {
            unimplemented!()
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
