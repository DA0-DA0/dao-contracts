use cosmwasm_std::{coins, BankMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use osmo_bindings::OsmosisMsg;

use crate::error::ContractError;
use crate::helpers::{check_bool_allowance, check_is_contract_owner};
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
    // validate that to_address is a valid address
    deps.api.addr_validate(&to_address)?;

    // don't allow minting of 0 coins
    if amount.is_zero() {
        return Result::Err(ContractError::ZeroAmount {});
    }

    // decrease minter allowance
    // if minter allowance goes negative, throw error
    MINTER_ALLOWANCES.update(deps.storage, &info.sender, |allowance| {
        allowance
            .unwrap_or_else(Uint128::zero)
            .checked_sub(amount)
            .map_err(StdError::overflow)
    })?;

    // get token denom from contract config
    let denom = CONFIG.load(deps.storage)?.denom;

    // create tokenfactory MsgMint which mints coins to the contract address
    let mint_tokens_msg =
        OsmosisMsg::mint_contract_tokens(denom.clone(), amount, env.contract.address.into_string());

    // send newly minted coins from contract to designated recipient
    let send_tokens_msg = BankMsg::Send {
        to_address: to_address.clone(),
        amount: coins(amount.u128(), denom),
    };

    // dispatch msgs
    Ok(Response::new()
        .add_message(mint_tokens_msg)
        .add_message(send_tokens_msg)
        .add_attribute("action", "mint")
        .add_attribute("to", to_address)
        .add_attribute("amount", amount))
}

pub fn burn(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // don't allow burning of 0 coins
    if amount.is_zero() {
        return Result::Err(ContractError::ZeroAmount {});
    }

    // decrease burner allowance
    // if burner allowance goes negative, throw error
    BURNER_ALLOWANCES.update(deps.storage, &info.sender, |allowance| {
        allowance
            .unwrap_or_else(Uint128::zero)
            .checked_sub(amount)
            .map_err(StdError::overflow)
    })?;

    // get token denom from contract config
    let denom = CONFIG.load(deps.storage)?.denom;

    // create tokenfactory MsgBurn which burns coins from the contract address
    // NOTE: this requires the contract to own the tokens already
    let burn_tokens_msg = OsmosisMsg::burn_contract_tokens(denom, amount, "".to_string());

    // dispatch msg
    Ok(Response::new()
        .add_message(burn_tokens_msg)
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount))
}

pub fn change_contract_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to change owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // validate that new owner is a valid address
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;

    // update the contract owner in the contract config
    CONFIG.update(
        deps.storage,
        |mut config: Config| -> Result<Config, ContractError> {
            config.owner = new_owner_addr;
            Ok(config)
        },
    )?;

    // return OK
    Ok(Response::new()
        .add_attribute("action", "change_contract_owner")
        .add_attribute("new_owner", new_owner))
}

pub fn change_tokenfactory_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to change tokenfactory admin
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // validate that the new admin is a valid address
    let new_admin_addr = deps.api.addr_validate(&new_admin)?;

    // construct tokenfactory change admin msg
    let change_admin_msg = OsmosisMsg::ChangeAdmin {
        denom: CONFIG.load(deps.storage)?.denom,
        new_admin_address: new_admin_addr.into(),
    };

    // dispatch change admin msg
    Ok(Response::new()
        .add_message(change_admin_msg)
        .add_attribute("action", "change_tokenfactory_admin")
        .add_attribute("new_admin", new_admin))
}

pub fn set_blacklister(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to set blacklister permission
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // set blacklister status
    // NOTE: Does not check if new status is same as old status
    BLACKLISTER_ALLOWANCES.save(deps.storage, &deps.api.addr_validate(&address)?, &status)?;

    // Return OK
    Ok(Response::new()
        .add_attribute("action", "set_blacklister")
        .add_attribute("blacklister", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_freezer(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to set freezer permission
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // set freezer status
    // NOTE: Does not check if new status is same as old status
    FREEZER_ALLOWANCES.save(deps.storage, &deps.api.addr_validate(&address)?, &status)?;

    // return OK
    Ok(Response::new()
        .add_attribute("action", "set_freezer")
        .add_attribute("freezer", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_burner(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to set burner allowance
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // update allowance of burner
    // validate that burner is a valid address
    BURNER_ALLOWANCES.save(deps.storage, &deps.api.addr_validate(&address)?, &allowance)?;

    // return OK
    Ok(Response::new()
        .add_attribute("action", "set_burner")
        .add_attribute("burner", address)
        .add_attribute("allowance", allowance))
}

pub fn set_minter(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to set minter allowance
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // update allowance of minter
    // validate that minter is a valid address
    MINTER_ALLOWANCES.save(deps.storage, &deps.api.addr_validate(&address)?, &allowance)?;

    // return OK
    Ok(Response::new()
        .add_attribute("action", "set_minter")
        .add_attribute("minter", address)
        .add_attribute("amount", allowance))
}

pub fn freeze(
    deps: DepsMut,
    info: MessageInfo,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // check to make sure that the sender has freezer permissions
    check_bool_allowance(deps.as_ref(), info, FREEZER_ALLOWANCES)?;

    // Update config frozen status
    // NOTE: Does not check if new status is same as old status
    CONFIG.update(
        deps.storage,
        |mut config: Config| -> Result<_, ContractError> {
            config.is_frozen = status;
            Ok(config)
        },
    )?;

    // return OK
    Ok(Response::new()
        .add_attribute("action", "freeze")
        .add_attribute("status", status.to_string()))
}

pub fn blacklist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // check to make sure that the sender has blacklister permissions
    check_bool_allowance(deps.as_ref(), info, BLACKLISTER_ALLOWANCES)?;

    // update blacklisted status
    // validate that blacklisteed is a valid address
    // NOTE: Does not check if new status is same as old status
    BLACKLISTED_ADDRESSES.save(deps.storage, &deps.api.addr_validate(&address)?, &status)?;

    // return OK
    Ok(Response::new()
        .add_attribute("action", "blacklist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}
