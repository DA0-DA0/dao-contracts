#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, LookUpDaoResponse, LookUpNameResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, CONFIG, DAO_TO_NAME, NAME_TO_DAO};

const CONTRACT_NAME: &str = "crates.io:cw-name-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let payment_token_address = deps.api.addr_validate(&msg.payment_token_address)?;
    let admin = deps.api.addr_validate(&msg.admin)?;

    if msg.payment_amount.is_zero() {
        return Err(ContractError::Unauthorized {});
    }

    let config = Config {
        admin,
        payment_token_address,
        payment_amount: msg.payment_amount,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(wrapped) => execute_receive(deps, env, info, wrapped),
        ExecuteMsg::UpdateConfig {
            new_payment_token_address,
            new_admin,
            new_payment_amount,
        } => execute_update_config(
            deps,
            env,
            info,
            new_payment_token_address,
            new_admin,
            new_payment_amount,
        ),
        ExecuteMsg::Revoke { name } => execute_revoke(deps, env, info, name),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapped: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // We only take payments from our specified token
    if info.sender != config.payment_token_address {
        return Err(ContractError::Unauthorized {});
    }

    // TODO: Is there a way we can verify this is a DAO
    let sender = wrapped.sender;
    let amount = wrapped.amount;
    let msg: ReceiveMsg = from_binary(&wrapped.msg)?;

    match msg {
        ReceiveMsg::Register { name } => register_name(deps, env, sender, amount, name),
    }
}

pub fn register_name(
    deps: DepsMut,
    _env: Env,
    sender: String,
    amount: Uint128,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if amount < config.payment_amount {
        // TODO: Improve error here
        return Err(ContractError::Unauthorized {});
    }

    // this is the DAO
    let sender = deps.api.addr_validate(&sender)?;

    if NAME_TO_DAO.has(deps.storage, name.clone()) {
        // TODO: Improve error here
        return Err(ContractError::Unauthorized {});
    }

    NAME_TO_DAO.save(deps.storage, name.clone(), &sender)?;
    DAO_TO_NAME.save(deps.storage, sender, &name)?;

    Ok(Response::new())
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_payment_token_address: Option<String>,
    new_admin: Option<String>,
    new_payment_amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let new_payment_token_address =
        new_payment_token_address.unwrap_or(config.payment_token_address.to_string());
    let payment_amount = new_payment_amount.unwrap_or(config.payment_amount);
    let new_admin = new_admin.unwrap_or(config.admin.to_string());

    // Validate addresses
    let admin = deps.api.addr_validate(&new_admin)?;
    let payment_token_address = deps.api.addr_validate(&new_payment_token_address)?;

    config.payment_amount = payment_amount;
    config.admin = admin;
    config.payment_token_address = payment_token_address;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let dao = NAME_TO_DAO.load(deps.storage, name.clone())?;

    // Only name owner and overall name admin can revoke
    if info.sender != dao && info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    NAME_TO_DAO.remove(deps.storage, name);
    DAO_TO_NAME.remove(deps.storage, dao);

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::LookUpDao { dao } => query_look_up_dao(deps, dao),
        QueryMsg::LookUpName { name } => query_look_up_name(deps, name),
    }
}

pub fn query_look_up_dao(deps: Deps, dao: String) -> StdResult<Binary> {
    let dao = deps.api.addr_validate(&dao)?;
    let name = DAO_TO_NAME.may_load(deps.storage, dao)?;
    to_binary(&LookUpDaoResponse { name })
}

pub fn query_look_up_name(deps: Deps, name: String) -> StdResult<Binary> {
    let dao = NAME_TO_DAO.may_load(deps.storage, name)?;
    to_binary(&LookUpNameResponse { dao })
}
