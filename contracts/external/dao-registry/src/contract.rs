#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Timestamp, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_utils::{must_pay, nonpayable};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::registration::Registration;
use crate::state::{Config, CONFIG, NAMES, PENDING_REGISTRATIONS, REGISTRATIONS};

const CONTRACT_NAME: &str = "crates.io:dao-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    let fee_denom = msg.fee_denom.into_checked(deps.as_ref())?;
    CONFIG.save(
        deps.storage,
        &Config {
            fee_amount: msg.fee_amount,
            fee_denom,
            registration_period: msg.registration_period,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::Receive(msg) => execute_receive_cw20(env, deps, info, msg),
        ExecuteMsg::Register { name, address } => execute_register(env, deps, info, name, address),
        ExecuteMsg::Renew {} => execute_renew(env, deps, info),
        ExecuteMsg::Approve { address } => execute_approve(env, deps, info, address),
        ExecuteMsg::Reject { address } => execute_reject(env, deps, info, address),
        ExecuteMsg::Revoke { name } => execute_revoke(env, deps, info, name),
        ExecuteMsg::UpdateExpiration { name, expiration } => {
            execute_update_expiration(deps, info, name, expiration)
        }
        ExecuteMsg::UpdateConfig {
            fee_amount,
            fee_denom,
            registration_period,
        } => execute_update_config(deps, info, fee_amount, fee_denom, registration_period),
    }
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    match action {
        // There must be an owner.
        cw_ownable::Action::RenounceOwnership {} => Err(ContractError::CannotRenounceOwnership),
        // Allow transfering and accepting ownership.
        _ => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default().add_attributes(ownership.into_attributes()))
        }
    }
}

pub fn execute_receive_cw20(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only accepts cw20 tokens.
    nonpayable(&info)?;

    let msg: ReceiveMsg = from_binary(&receive_msg.msg)?;

    // Validate amount and denom.
    let config = CONFIG.load(deps.storage)?;
    if !config.fee_denom.is_cw20(&info.sender) {
        return Err(ContractError::WrongDenom);
    }
    if receive_msg.amount != config.fee_amount {
        return Err(ContractError::WrongAmount);
    }

    match msg {
        ReceiveMsg::Register { name } => try_register(env, deps, info.sender, name, false),
        ReceiveMsg::Renew {} => try_renew(env, deps, info.sender),
    }
}

pub fn execute_register(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    address: Option<String>,
) -> Result<Response, ContractError> {
    // If explicit address passed, validate sender is the owner. This will
    // bypass the fee and approval step. Otherwise, require fee payment.
    let owner_override = address.is_some();
    if owner_override {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;
        // Ensure owner cannot pay any fee.
        nonpayable(&info)?;
    } else {
        let config = CONFIG.load(deps.storage)?;
        // Validate amount and denom. Only accepts native tokens.
        match config.fee_denom {
            CheckedDenom::Native(ref denom) => {
                let sent = must_pay(&info, denom)?;
                if config.fee_amount != sent {
                    return Err(ContractError::WrongAmount {});
                }
            }
            CheckedDenom::Cw20(_) => {
                // Cw20 registration happens via ExecuteMsg::Receive.
                return Err(ContractError::WrongDenom {});
            }
        };
    }

    // If address passed in (owner override), validate it. Otherwise use the
    // sender.
    let registration_address = if let Some(address) = address {
        deps.api.addr_validate(&address)?
    } else {
        info.sender
    };

    try_register(env, deps, registration_address, name, owner_override)
}

pub fn execute_renew(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Validate amount and denom. Only accepts native tokens.
    match config.fee_denom {
        CheckedDenom::Native(ref denom) => {
            let sent = must_pay(&info, denom)?;
            if config.fee_amount != sent {
                return Err(ContractError::WrongAmount {});
            }
        }
        CheckedDenom::Cw20(_) => {
            // Cw20 registration happens via ExecuteMsg::Receive.
            nonpayable(&info)?;
        }
    };

    try_renew(env, deps, info.sender)
}

pub fn execute_approve(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    // Validate address.
    let address = deps.api.addr_validate(&address)?;

    // Only owner can approve.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Get pending registration.
    let mut registration =
        pending_or_active_registration_for_address(&env, &deps.as_ref(), &address)?
            .ok_or(ContractError::NoPendingRegistrationFound)?;
    if !registration.is_pending() {
        return Err(ContractError::NoPendingRegistrationFound);
    }

    // Forward the fee to the owner.
    let owner = get_owner(&deps.as_ref())?;
    let transfer_fee_msg = registration.get_transfer_msg(&owner, None)?;

    // Approve the registration.
    registration.approve(&env, deps)?;

    Ok(Response::new()
        .add_message(transfer_fee_msg)
        .add_attribute("method", "approve")
        .add_attribute("address", address.to_string())
        .add_attribute("name", registration.name))
}

pub fn execute_reject(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    // Validate address.
    let address = deps.api.addr_validate(&address)?;

    // Only owner can reject.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Get pending registration.
    let mut registration =
        pending_or_active_registration_for_address(&env, &deps.as_ref(), &address)?
            .ok_or(ContractError::NoPendingRegistrationFound)?;
    if !registration.is_pending() {
        return Err(ContractError::NoPendingRegistrationFound);
    }

    // Send the fee back to the address.
    let transfer_fee_msg = registration.get_transfer_msg(&registration.address, None)?;

    // Reject the registration.
    registration.reject(&env, deps)?;

    Ok(Response::new()
        .add_message(transfer_fee_msg)
        .add_attribute("method", "reject")
        .add_attribute("address", address.to_string())
        .add_attribute("name", registration.name))
}

pub fn execute_revoke(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    // Only owner can revoke.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Get active registration.
    let mut registration = active_registration_for_name(&env, &deps.as_ref(), &name)?
        .ok_or(ContractError::NoRegistrationFound)?;

    // Reject the registration.
    registration.revoke(&env, deps)?;

    Ok(Response::new()
        .add_attribute("method", "revoke")
        .add_attribute("address", registration.address.to_string())
        .add_attribute("name", registration.name))
}

pub fn execute_update_expiration(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    expiration: Timestamp,
) -> Result<Response, ContractError> {
    // Only owner can update expiration.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Get registration.
    let mut registration =
        registration_for_name(&deps.as_ref(), &name)?.ok_or(ContractError::NoRegistrationFound)?;

    // Update the expiration.
    registration.expiration = expiration;
    REGISTRATIONS.save(deps.storage, registration.address.clone(), &registration)?;

    Ok(Response::new()
        .add_attribute("method", "update_expiration")
        .add_attribute("address", registration.address.to_string())
        .add_attribute("name", registration.name))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_amount: Option<Uint128>,
    fee_denom: Option<UncheckedDenom>,
    registration_period: Option<Timestamp>,
) -> Result<Response, ContractError> {
    // Only owner can update config.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let checked_denom = fee_denom
        .map(|unchecked| unchecked.into_checked(deps.as_ref()))
        .transpose()?;

    // Update config.
    let config = CONFIG.update(deps.storage, |config| {
        StdResult::<Config>::Ok(Config {
            fee_amount: fee_amount.unwrap_or(config.fee_amount),
            fee_denom: checked_denom.unwrap_or(config.fee_denom),
            registration_period: registration_period.unwrap_or(config.registration_period),
        })
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("fee_amount", config.fee_amount.to_string())
        .add_attribute("fee_denom", config.fee_denom.to_string())
        .add_attribute(
            "registration_period",
            config.registration_period.to_string(),
        ))
}

pub fn try_register(
    env: Env,
    deps: DepsMut,
    address: Addr,
    name: String,
    // If true, the owner is registering on behalf of the address. Don't
    // transfer any fee in this case.
    owner_override: bool,
) -> Result<Response, ContractError> {
    // Validate DAO not already pending registration or registered.
    if let Some(registration) =
        pending_or_active_registration_for_address(&env, &deps.as_ref(), &address)?
    {
        if registration.is_pending() {
            return Err(ContractError::RegistrationPending);
        } else {
            return Err(ContractError::AlreadyRegistered);
        }
    }

    // Validate name not already registered.
    if active_registration_for_name(&env, &deps.as_ref(), &name)?.is_some() {
        return Err(ContractError::NameAlreadyRegistered);
    }

    // TODO: Validate name.
    if name.len() < 3 || name.len() > 32 {
        return Err(ContractError::InvalidName);
    }

    // Register the DAO.
    let config = CONFIG.load(deps.storage)?;
    let mut registration = Registration::new(address.clone(), name.clone(), config);
    REGISTRATIONS.save(deps.storage, address.clone(), &registration)?;
    PENDING_REGISTRATIONS.save(deps.storage, address.clone(), &registration)?;

    // If owner is registering on behalf of the address, registration is
    // approved immediately and there is no fee to send anywhere.
    if owner_override {
        registration.approve(&env, deps)?;
    }

    Ok(Response::new()
        .add_attribute("method", "register")
        .add_attribute("address", address.to_string())
        .add_attribute("name", name)
        .add_attribute("expiration", registration.expiration.nanos().to_string()))
}

pub fn try_renew(env: Env, deps: DepsMut, address: Addr) -> Result<Response, ContractError> {
    // Validate DAO is registered.
    let mut registration =
        match pending_or_active_registration_for_address(&env, &deps.as_ref(), &address)? {
            Some(registration) => registration,
            None => return Err(ContractError::NoRegistrationFound),
        };

    // Validate registration is still active. Otherwise it is pending.
    if !registration.is_active(&env) {
        return Err(ContractError::NoRegistrationFound);
    }

    // Validate registration can be renewed.
    if !registration.is_renewable(&env, &deps)? {
        return Err(ContractError::RegistrationAlreadyRenewed);
    }

    // Renew the registration.
    let config = CONFIG.load(deps.storage)?;
    registration.expiration = registration
        .expiration
        .plus_nanos(config.registration_period.nanos());
    REGISTRATIONS.save(deps.storage, address.clone(), &registration)?;

    // Pass the fee through to the owner.
    let owner = get_owner(&deps.as_ref())?;
    let transfer_fee_msg = registration.get_transfer_msg(&owner, Some(config))?;

    Ok(Response::new()
        .add_message(transfer_fee_msg)
        .add_attribute("method", "renew")
        .add_attribute("address", address.to_string())
        .add_attribute("name", registration.name)
        .add_attribute("expiration", registration.expiration.nanos().to_string()))
}

// Get the pending or active registration for the address. This will return None
// if the address is not registered or if the most recent registration has
// expired.
fn pending_or_active_registration_for_address(
    env: &Env,
    deps: &Deps,
    address: &Addr,
) -> StdResult<Option<Registration>> {
    let registration = REGISTRATIONS
        .may_load(deps.storage, address.clone())?
        .and_then(|registration| {
            if registration.is_pending() || registration.is_active(env) {
                Some(registration)
            } else {
                None
            }
        });
    Ok(registration)
}

// Get the registration for the given name. This will return None if the name
// has never been registered or if the most recent registerer registered a
// different name since registering this one.
fn registration_for_name(deps: &Deps, name: &String) -> StdResult<Option<Registration>> {
    let registration = NAMES
        .may_load(deps.storage, name.to_string())?
        .map(|addr| REGISTRATIONS.load(deps.storage, addr))
        .transpose()?
        .and_then(|registration| {
            if registration.name == *name {
                Some(registration)
            } else {
                None
            }
        });
    Ok(registration)
}

// Get the active registration for the given name. This will return None if the
// name is not registered or if the most recent registration has expired.
fn active_registration_for_name(
    env: &Env,
    deps: &Deps,
    name: &String,
) -> StdResult<Option<Registration>> {
    let registration = registration_for_name(deps, name)?.and_then(|registration| {
        if registration.is_active(env) {
            Some(registration)
        } else {
            None
        }
    });
    Ok(registration)
}

pub fn get_owner(deps: &Deps) -> StdResult<Addr> {
    Ok(cw_ownable::get_ownership(deps.storage)
        .map(|ownership: cw_ownable::Ownership<Addr>| ownership.owner)?
        .unwrap())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Registration { address } => to_binary(
            &REGISTRATIONS
                .may_load(deps.storage, deps.api.addr_validate(&address)?)?
                .and_then(|registration| {
                    if registration.is_active(&env) {
                        Some(registration)
                    } else {
                        None
                    }
                }),
        ),
        QueryMsg::Resolve { name } => to_binary(&active_registration_for_name(&env, &deps, &name)?),
        QueryMsg::PendingRegistration { address } => to_binary(
            &pending_or_active_registration_for_address(
                &env,
                &deps,
                &deps.api.addr_validate(&address)?,
            )?
            .and_then(|registration| {
                if registration.is_pending() {
                    Some(registration)
                } else {
                    None
                }
            }),
        ),
        QueryMsg::MostRecentRegistration { address } => {
            to_binary(&REGISTRATIONS.may_load(deps.storage, deps.api.addr_validate(&address)?)?)
        }
    }
}
