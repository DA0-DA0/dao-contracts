use cosmwasm_std::{coins, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128};
use osmo_bindings::OsmosisMsg;
use osmosis_std::types::cosmos::bank::v1beta1::Metadata;
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{MsgBurn, MsgSetDenomMetadata};

use crate::error::ContractError;
use crate::helpers::{check_bool_allowance, check_is_contract_owner};
use crate::state::{
    BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES, BURNER_ALLOWANCES, DENOM, FREEZER_ALLOWANCES,
    IS_FROZEN, MINTER_ALLOWANCES, OWNER,
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
        return Err(ContractError::ZeroAmount {});
    }

    // decrease minter allowance
    let allowance = MINTER_ALLOWANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(Uint128::zero);

    // if minter allowance goes negative, throw error
    let updated_allowance = allowance
        .checked_sub(amount)
        .map_err(|_| ContractError::not_enough_mint_allowance(amount, allowance))?;

    // if minter allowance goes 0, remove from storage
    if updated_allowance.is_zero() {
        MINTER_ALLOWANCES.remove(deps.storage, &info.sender);
    } else {
        MINTER_ALLOWANCES.save(deps.storage, &info.sender, &updated_allowance)?;
    }

    // get token denom from contract
    let denom = DENOM.load(deps.storage)?;

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
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    address: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // don't allow burning of 0 coins
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    // decrease burner allowance
    let allowance = BURNER_ALLOWANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(Uint128::zero);

    // if burner allowance goes negative, throw error
    let updated_allowance = allowance
        .checked_sub(amount)
        .map_err(|_| ContractError::not_enough_burn_allowance(amount, allowance))?;

    // if burner allowance goes 0, remove from storage
    if updated_allowance.is_zero() {
        BURNER_ALLOWANCES.remove(deps.storage, &info.sender);
    } else {
        BURNER_ALLOWANCES.save(deps.storage, &info.sender, &updated_allowance)?;
    }

    // get token denom from contract config
    let denom = DENOM.load(deps.storage)?;

    // create tokenfactory MsgBurn which burns coins from the contract address
    // NOTE: this requires the contract to own the tokens already
    let burn_from_address = deps.api.addr_validate(&address)?;
    let burn_tokens_msg: cosmwasm_std::CosmosMsg<OsmosisMsg> = MsgBurn {
        sender: env.contract.address.to_string(),
        amount: Some(Coin::new(amount.u128(), denom).into()),
        burn_from_address: burn_from_address.to_string(),
    }
    .into();

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
    OWNER.save(deps.storage, &new_owner_addr)?;

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
        denom: DENOM.load(deps.storage)?,
        new_admin_address: new_admin_addr.into(),
    };

    // dispatch change admin msg
    Ok(Response::new()
        .add_message(change_admin_msg)
        .add_attribute("action", "change_tokenfactory_admin")
        .add_attribute("new_admin", new_admin))
}

pub fn set_denom_metadata(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    metadata: Metadata,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // only allow current contract owner to set denom metadata
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "set_denom_metadata")
        .add_message(MsgSetDenomMetadata {
            sender: env.contract.address.to_string(),
            metadata: Some(metadata),
        }))
}

pub fn set_blacklister(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Only allow current contract owner to set blacklister permission
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    let address = deps.api.addr_validate(&address)?;

    // set blacklister status
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        BLACKLISTER_ALLOWANCES.save(deps.storage, &address, &status)?;
    } else {
        BLACKLISTER_ALLOWANCES.remove(deps.storage, &address);
    }

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

    let address = deps.api.addr_validate(&address)?;

    // set freezer status
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        FREEZER_ALLOWANCES.save(deps.storage, &address, &status)?;
    } else {
        FREEZER_ALLOWANCES.remove(deps.storage, &address);
    }

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

    // validate that burner is a valid address
    let address = deps.api.addr_validate(&address)?;

    // update allowance of burner
    // remove key from state if set to 0
    if allowance.is_zero() {
        BURNER_ALLOWANCES.remove(deps.storage, &address);
    } else {
        BURNER_ALLOWANCES.save(deps.storage, &address, &allowance)?;
    }

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

    // validate that minter is a valid address
    let address = deps.api.addr_validate(&address)?;

    // update allowance of minter
    // remove key from state if set to 0
    if allowance.is_zero() {
        MINTER_ALLOWANCES.remove(deps.storage, &address);
    } else {
        MINTER_ALLOWANCES.save(deps.storage, &address, &allowance)?;
    }

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
    IS_FROZEN.save(deps.storage, &status)?;

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

    let address = deps.api.addr_validate(&address)?;

    // update blacklisted status
    // validate that blacklisteed is a valid address
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        BLACKLISTED_ADDRESSES.save(deps.storage, &address, &status)?;
    } else {
        BLACKLISTED_ADDRESSES.remove(deps.storage, &address);
    }

    // return OK
    Ok(Response::new()
        .add_attribute("action", "blacklist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}
