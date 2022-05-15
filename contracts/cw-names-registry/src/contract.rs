#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, TokenInfoResponse};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, IsNameAvailableToRegisterResponse, LookUpDaoByNameResponse,
    LookUpNameByDaoResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, PaymentInfo, CONFIG, DAO_TO_NAME, NAME_TO_DAO, RESERVED_NAMES};

const CONTRACT_NAME: &str = "crates.io:cw-name-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn assert_cw20(deps: Deps, cw20_addr: &Addr) -> Result<(), ContractError> {
    let _resp: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(cw20_addr, &cw20_base::msg::QueryMsg::TokenInfo {})
        .map_err(|_err| ContractError::InvalidCw20 {})?;
    Ok(())
}

fn validate_payment_info(deps: Deps, payment_info: PaymentInfo) -> Result<(), ContractError> {
    match payment_info {
        PaymentInfo::Cw20Payment {
            token_address,
            payment_amount,
        } => {
            if payment_amount.is_zero() {
                return Err(ContractError::InvalidPaymentAmount {});
            }

            // Validate it is a valid CW20 address
            let payment_token_address = deps.api.addr_validate(&token_address)?;
            assert_cw20(deps, &payment_token_address)?;
        }
        PaymentInfo::NativePayment { payment_amount, .. } => {
            if payment_amount.is_zero() {
                return Err(ContractError::InvalidPaymentAmount {});
            }
        }
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_payment_info(deps.as_ref(), msg.payment_info.clone())?;
    let validated_admin = deps.api.addr_validate(&msg.admin)?;
    let config = Config {
        admin: validated_admin,
        payment_info: msg.payment_info,
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
        ExecuteMsg::RegisterName { name } => execute_register_name_native(deps, env, info, name),
        ExecuteMsg::UpdateConfig {
            new_admin,
            new_payment_info,
        } => execute_update_config(deps, env, info, new_admin, new_payment_info),
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

    match config.payment_info {
        PaymentInfo::NativePayment { .. } => Err(ContractError::Unauthorized {}),
        PaymentInfo::Cw20Payment {
            token_address,
            payment_amount,
        } => {
            if info.sender != token_address {
                return Err(ContractError::UnrecognisedCw20 {});
            }

            let sender = wrapped.sender;
            let amount = wrapped.amount;
            let msg: ReceiveMsg = from_binary(&wrapped.msg)?;

            match msg {
                ReceiveMsg::Register { name } => {
                    register_name(deps, env, sender, amount, name, payment_amount)
                }
            }
        }
    }
}

pub fn execute_register_name_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match config.payment_info {
        PaymentInfo::Cw20Payment { .. } => Err(ContractError::InvalidPayment {}),
        PaymentInfo::NativePayment {
            token_denom,
            payment_amount,
        } => {
            let token_idx = info.funds.iter().position(|c| c.denom == token_denom);
            if token_idx.is_none() {
                return Err(ContractError::UnrecognisedNativeToken {});
            }

            let coins = &info.funds[token_idx.unwrap()];

            register_name(
                deps,
                env,
                info.sender.to_string(),
                coins.amount,
                name,
                payment_amount,
            )
        }
    }
}

pub fn register_name(
    deps: DepsMut,
    _env: Env,
    sender: String,
    amount_sent: Uint128,
    name: String,
    payment_amount_to_register_name: Uint128,
) -> Result<Response, ContractError> {
    if amount_sent != payment_amount_to_register_name {
        return Err(ContractError::IncorrectPaymentAmount {});
    }

    // We expect this to be a DAO that is registering a name
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

    let config = CONFIG.load(deps.storage)?;

    let msg = match config.payment_info {
        PaymentInfo::NativePayment { token_denom, .. } => CosmosMsg::Bank(BankMsg::Send {
            to_address: config.admin.to_string(),
            amount: coins(amount_sent.u128(), token_denom),
        }),
        PaymentInfo::Cw20Payment { token_address, .. } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_address,
            msg: to_binary(&cw20_base::msg::ExecuteMsg::Transfer {
                recipient: config.admin.to_string(),
                amount: amount_sent,
            })?,
            funds: vec![],
        }),
    };

    Ok(Response::new()
        .add_attribute("action", "register_name")
        .add_message(msg))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_admin: Option<String>,
    new_payment_info: Option<PaymentInfo>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let new_payment_info = new_payment_info.unwrap_or_else(|| config.clone().payment_info);
    let new_admin = new_admin.unwrap_or_else(|| config.admin.to_string());

    // Validate admin address
    let admin = deps.api.addr_validate(&new_admin)?;

    // Validate payment info
    validate_payment_info(deps.as_ref(), new_payment_info.clone())?;

    config.admin = admin;
    config.payment_info = new_payment_info;

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

    Ok(Response::new().add_attribute("action", "revoke_name"))
}

pub fn execute_reserve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only the admin can reserve names
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Check if name is already taken
    if NAME_TO_DAO.has(deps.storage, name.clone()) {
        return Err(ContractError::NameAlreadyTaken {});
    }

    if RESERVED_NAMES.has(deps.storage, name.clone()) {
        return Err(ContractError::NameReserved {});
    }

    RESERVED_NAMES.save(deps.storage, name, &Empty {})?;

    Ok(Response::new().add_attribute("action", "reserve_name"))
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

    Ok(Response::new().add_attribute("action", "transfer_reservation"))
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
    to_binary(&LookUpNameByDaoResponse { name })
}

pub fn query_look_up_dao_by_name(deps: Deps, name: String) -> StdResult<Binary> {
    let dao = NAME_TO_DAO.may_load(deps.storage, name)?;
    to_binary(&LookUpDaoByNameResponse { dao })
}

pub fn query_is_name_available_to_register(deps: Deps, name: String) -> StdResult<Binary> {
    let reserved = RESERVED_NAMES.has(deps.storage, name.clone());
    let taken = NAME_TO_DAO.has(deps.storage, name);
    to_binary(&IsNameAvailableToRegisterResponse { taken, reserved })
}
