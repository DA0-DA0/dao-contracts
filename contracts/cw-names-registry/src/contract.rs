#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, IsNameAvailableToRegisterResponse, LookUpDaoResponse,
    LookUpNameResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, CONFIG, DAO_TO_NAME, NAME_TO_DAO, RESERVED_NAMES};

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

    if msg.payment_amount_to_register_name.is_zero() {
        return Err(ContractError::InvalidPaymentAmount {});
    }

    let config = Config {
        admin,
        payment_token_address,
        payment_amount_to_register_name: msg.payment_amount_to_register_name,
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
        ExecuteMsg::Reserve { name } => execute_reserve(deps, env, info, name),
        ExecuteMsg::TransferReservation { name, dao } => {
            execute_transfer_reservation(deps, env, info, name, dao)
        }
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
    if amount < config.payment_amount_to_register_name {
        return Err(ContractError::InsufficientFunds {});
    }

    // We expect this to be a DAO that is registering a name
    // TODO: Validate it is a DAO
    let sender = deps.api.addr_validate(&sender)?;

    if RESERVED_NAMES.has(deps.storage, name.clone()) {
        return Err(ContractError::NameReserved {});
    }

    if NAME_TO_DAO.has(deps.storage, name.clone()) {
        return Err(ContractError::NameAlreadyTaken {});
    }

    if DAO_TO_NAME.has(deps.storage, sender.clone()) {
        return Err(ContractError::AlreadyRegisteredName {});
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
        new_payment_token_address.unwrap_or_else(|| config.payment_token_address.to_string());
    let payment_amount = new_payment_amount.unwrap_or(config.payment_amount_to_register_name);
    let new_admin = new_admin.unwrap_or_else(|| config.admin.to_string());

    // Validate payment amount
    if payment_amount.is_zero() {
        return Err(ContractError::InvalidPaymentAmount {});
    }

    // Validate addresses
    let admin = deps.api.addr_validate(&new_admin)?;
    let payment_token_address = deps.api.addr_validate(&new_payment_token_address)?;

    config.payment_amount_to_register_name = payment_amount;
    config.admin = admin;
    config.payment_token_address = payment_token_address;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_revoke(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if !NAME_TO_DAO.has(deps.storage, name.clone()) {
        return Err(ContractError::NameNotRegistered {});
    }

    let dao = NAME_TO_DAO.load(deps.storage, name.clone())?;

    // Only name owner and overall name admin can revoke
    if info.sender != dao && info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    NAME_TO_DAO.remove(deps.storage, name);
    DAO_TO_NAME.remove(deps.storage, dao);

    Ok(Response::new())
}

pub fn execute_reserve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Check if name is already taken
    if NAME_TO_DAO.has(deps.storage, name.clone()) {
        return Err(ContractError::NameAlreadyTaken {});
    }

    if RESERVED_NAMES.has(deps.storage, name.clone()) {
        return Err(ContractError::NameReserved {});
    }

    // Only the admin can reserve names
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    RESERVED_NAMES.save(deps.storage, name, &Empty {})?;

    Ok(Response::new())
}

pub fn execute_transfer_reservation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    dao: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Validate DAO
    let dao = deps.api.addr_validate(&dao)?;

    // Only the admin can transfer reserved names
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Check if DAO already has a name
    if DAO_TO_NAME.has(deps.storage, dao.clone()) {
        return Err(ContractError::AlreadyRegisteredName {});
    }

    if !RESERVED_NAMES.has(deps.storage, name.clone()) {
        return Err(ContractError::NameNotReserved {});
    }

    DAO_TO_NAME.save(deps.storage, dao.clone(), &name)?;
    NAME_TO_DAO.save(deps.storage, name.clone(), &dao)?;
    RESERVED_NAMES.remove(deps.storage, name);

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::LookUpNameByDao { dao } => query_look_up_name_by_dao(deps, dao),
        QueryMsg::LookUpDaoByName { name } => query_look_up_dao_by_name(deps, name),
        QueryMsg::IsNameAvailableToRegister { name } => {
            query_is_name_available_to_register(deps, name)
        }
    }
}

pub fn query_look_up_name_by_dao(deps: Deps, dao: String) -> StdResult<Binary> {
    let dao = deps.api.addr_validate(&dao)?;
    let name = DAO_TO_NAME.may_load(deps.storage, dao)?;
    to_binary(&LookUpDaoResponse { name })
}

pub fn query_look_up_dao_by_name(deps: Deps, name: String) -> StdResult<Binary> {
    let dao = NAME_TO_DAO.may_load(deps.storage, name)?;
    to_binary(&LookUpNameResponse { dao })
}

pub fn query_is_name_available_to_register(deps: Deps, name: String) -> StdResult<Binary> {
    let reserved = RESERVED_NAMES.has(deps.storage, name.clone());
    let taken = NAME_TO_DAO.has(deps.storage, name);
    to_binary(&IsNameAvailableToRegisterResponse { taken, reserved })
}
