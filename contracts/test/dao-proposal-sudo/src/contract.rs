#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{DAO, ROOT},
};

const CONTRACT_NAME: &str = "crates.io:cw-govmod-sudo";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let root = deps.api.addr_validate(&msg.root)?;
    ROOT.save(deps.storage, &root)?;
    DAO.save(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("root", root))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Execute { msgs } => execute_execute(deps.as_ref(), info.sender, msgs),
    }
}

pub fn execute_execute(
    deps: Deps,
    sender: Addr,
    msgs: Vec<CosmosMsg>,
) -> Result<Response, ContractError> {
    let root = ROOT.load(deps.storage)?;
    let dao = DAO.load(deps.storage)?;

    if sender != root {
        return Err(ContractError::Unauthorized {});
    }

    let msg = WasmMsg::Execute {
        contract_addr: dao.to_string(),
        msg: to_json_binary(&dao_interface::msg::ExecuteMsg::ExecuteProposalHook { msgs })?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_attribute("action", "execute_execute")
        .add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => query_admin(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::Info {} => query_info(deps),
    }
}

pub fn query_admin(deps: Deps) -> StdResult<Binary> {
    to_json_binary(&ROOT.load(deps.storage)?)
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    to_json_binary(&DAO.load(deps.storage)?)
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}
