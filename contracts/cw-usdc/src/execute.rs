#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use osmo_bindings::OsmosisMsg;

use crate::error::ContractError;
use crate::helpers::{
    check_bool_allowance, check_funds, check_is_contract_owner, set_bool_allowance,
    set_int_allowance,
};
use crate::state::{
    Config, BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES, BURNER_ALLOWANCES, CONFIG,
    FREEZER_ALLOWANCES, MINTER_ALLOWANCES,
};

pub fn mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_address: String,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    deps.api.addr_validate(&to_address)?;
    let denom = CONFIG.load(deps.storage).unwrap().denom;

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(ContractError::ZeroAmount {});
    }

    let _allowance = MINTER_ALLOWANCES.update(
        deps.storage,
        &info.sender,
        |allowance| -> StdResult<Uint128> {
            Ok(allowance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    let mint_tokens_msg =
        OsmosisMsg::mint_contract_tokens(denom, amount, env.contract.address.into_string());

    // TODO: Second msg that sends tokens to the to_address

    Ok(Response::new()
        .add_attribute("method", "mint_tokens")
        .add_message(mint_tokens_msg))
}

pub fn burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let denom = CONFIG.load(deps.storage).unwrap().denom;

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(ContractError::ZeroAmount {});
    }

    // Contract needs to own the coins it wants to burn
    check_funds(denom.clone(), &info.funds, amount)?;

    let _allowance = BURNER_ALLOWANCES.update(
        deps.storage,
        &info.sender.clone(),
        |allowance| -> StdResult<Uint128> {
            Ok(allowance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    // TODO execute actual BurnMsg -> needs to be the contract address or maybe need to include in info.funds... see whats possible
    // burns tokens that are owned by this contract.
    let burn_tokens_msg = OsmosisMsg::burn_contract_tokens(denom, amount, "".to_string());

    Ok(Response::new()
        .add_attribute("method", "execute_burn")
        .add_attribute("amount", amount.to_string())
        .add_message(burn_tokens_msg))
}

pub fn change_contract_owner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // TODO: check using the comment below and save state instead of update
    // check_is_contract_owner(deps.as_ref(), info.sender)?;
    let val_address = deps.api.addr_validate(address.as_str())?;

    CONFIG.update(
        deps.storage,
        |mut config: Config| -> Result<Config, ContractError> {
            if config.owner == info.sender {
                config.owner = val_address;
                return Ok(config);
            }

            return Err(ContractError::Unauthorized {});
        },
    )?;

    Ok(Response::new().add_attribute("method", "change_contract_owner"))
}

pub fn set_blacklister(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    set_bool_allowance(deps, &address, BLACKLISTER_ALLOWANCES, status)?;

    Ok(Response::new()
        .add_attribute("method", "set_blacklister")
        .add_attribute("blacklister", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_freezer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Check if sender is authorised to set freezer
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    set_bool_allowance(deps, &address, FREEZER_ALLOWANCES, status)?;

    Ok(Response::new()
        .add_attribute("method", "set_freezer")
        .add_attribute("freezer", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_burner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Set Burner allowance
    set_int_allowance(deps, BURNER_ALLOWANCES, &address, amount)?;

    Ok(Response::new()
        .add_attribute("method", "set_burner")
        .add_attribute("burner", address)
        .add_attribute("amount", amount))
}

pub fn set_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Set minter allowance
    set_int_allowance(deps, MINTER_ALLOWANCES, &address, amount)?;

    Ok(Response::new()
        .add_attribute("method", "set_minter")
        .add_attribute("minter", address)
        .add_attribute("amount", amount))
}

pub fn freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // check if the sender is allowed to freeze
    check_bool_allowance(&deps.as_ref(), info.clone(), FREEZER_ALLOWANCES)?;

    let config = CONFIG.load(deps.storage)?;
    if config.is_frozen == status {
        Err(ContractError::ContractFrozenStatusUnchanged { status })
    } else {
        CONFIG.update(
            deps.storage,
            |mut config: Config| -> Result<_, ContractError> {
                config.is_frozen = status;
                Ok(config)
            },
        )?;

        Ok(Response::new()
            .add_attribute("method", "execute_freeze")
            .add_attribute("status", status.to_string()))
    }
}

pub fn blacklist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_bool_allowance(&deps.as_ref(), info, BLACKLISTER_ALLOWANCES)?;

    // update blacklisted status
    BLACKLISTED_ADDRESSES.update(
        deps.storage,
        &deps.api.addr_validate(address.as_str())?,
        |mut stat| -> Result<_, ContractError> {
            stat = Some(status);
            Ok(status)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "blacklist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}
