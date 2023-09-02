use cosmwasm_std::{coins, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use osmosis_std::types::cosmos::bank::v1beta1::Metadata;
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgForceTransfer, MsgSetBeforeSendHook, MsgSetDenomMetadata,
};
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery};

use crate::error::ContractError;
use crate::helpers::{
    check_before_send_hook_features_enabled, check_bool_allowance, check_is_contract_owner,
};
use crate::state::{
    BEFORE_SEND_HOOK_FEATURES_ENABLED, BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES,
    BURNER_ALLOWANCES, DENOM, FREEZER_ALLOWANCES, IS_FROZEN, MINTER_ALLOWANCES, OWNER,
    WHITELISTED_ADDRESSES, WHITELISTER_ALLOWANCES,
};

pub fn mint(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    to_address: String,
    amount: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
    let mint_tokens_msg = TokenFactoryMsg::mint_contract_tokens(
        denom.clone(),
        amount,
        env.contract.address.into_string(),
    );

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
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    address: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
    let burn_tokens_msg: cosmwasm_std::CosmosMsg<TokenFactoryMsg> = MsgBurn {
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
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // validate that new owner is a valid address
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;

    // update the contract owner in the contract config
    OWNER.save(deps.storage, &new_owner_addr)?;

    Ok(Response::new()
        .add_attribute("action", "change_contract_owner")
        .add_attribute("new_owner", new_owner))
}

pub fn change_tokenfactory_admin(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change tokenfactory admin
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // validate that the new admin is a valid address
    let new_admin_addr = deps.api.addr_validate(&new_admin)?;

    // construct tokenfactory change admin msg
    let change_admin_msg = TokenFactoryMsg::ChangeAdmin {
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
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    metadata: Metadata,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

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

    Ok(Response::new()
        .add_attribute("action", "set_blacklister")
        .add_attribute("blacklister", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_whitelister(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to set blacklister permission
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    let address = deps.api.addr_validate(&address)?;

    // set blacklister status
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        WHITELISTER_ALLOWANCES.save(deps.storage, &address, &status)?;
    } else {
        WHITELISTER_ALLOWANCES.remove(deps.storage, &address);
    }

    // Return OK
    Ok(Response::new()
        .add_attribute("action", "set_blacklister")
        .add_attribute("blacklister", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_freezer(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

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

    Ok(Response::new()
        .add_attribute("action", "set_freezer")
        .add_attribute("freezer", address)
        .add_attribute("status", status.to_string()))
}

pub fn set_before_send_hook(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Return error if BeforeSendHook already enabled
    if BEFORE_SEND_HOOK_FEATURES_ENABLED.load(deps.storage)? {
        return Err(ContractError::BeforeSendHookAlreadyEnabled {});
    }

    // Load the Token Factory denom
    let denom = DENOM.load(deps.storage)?;

    // SetBeforeSendHook to this contract
    // this will trigger sudo endpoint before any bank send
    // which makes blacklisting / freezing possible
    let msg_set_beforesend_hook: CosmosMsg<TokenFactoryMsg> = MsgSetBeforeSendHook {
        sender: env.contract.address.to_string(),
        denom: denom.clone(),
        cosmwasm_address: env.contract.address.to_string(),
    }
    .into();

    // Enable BeforeSendHook features
    BEFORE_SEND_HOOK_FEATURES_ENABLED.save(deps.storage, &true)?;

    Ok(Response::new()
        .add_attribute("action", "set_before_send_hook")
        .add_message(msg_set_beforesend_hook))
}

pub fn set_burner(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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

    Ok(Response::new()
        .add_attribute("action", "set_burner")
        .add_attribute("burner", address)
        .add_attribute("allowance", allowance))
}

pub fn set_minter(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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

    Ok(Response::new()
        .add_attribute("action", "set_minter")
        .add_attribute("minter", address)
        .add_attribute("amount", allowance))
}

pub fn freeze(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // check to make sure that the sender has freezer permissions
    check_bool_allowance(deps.as_ref(), info, FREEZER_ALLOWANCES)?;

    // Update config frozen status
    // NOTE: Does not check if new status is same as old status
    IS_FROZEN.save(deps.storage, &status)?;

    Ok(Response::new()
        .add_attribute("action", "freeze")
        .add_attribute("status", status.to_string()))
}

pub fn blacklist(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // check to make sure that the sender has blacklister permissions
    check_bool_allowance(deps.as_ref(), info, BLACKLISTER_ALLOWANCES)?;

    let address = deps.api.addr_validate(&address)?;

    // Check this issuer contract is not blacklisting itself
    if address == env.contract.address {
        return Err(ContractError::CannotBlacklistSelf {});
    }

    // update blacklisted status
    // validate that blacklisteed is a valid address
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        BLACKLISTED_ADDRESSES.save(deps.storage, &address, &status)?;
    } else {
        BLACKLISTED_ADDRESSES.remove(deps.storage, &address);
    }

    Ok(Response::new()
        .add_attribute("action", "blacklist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}

pub fn whitelist(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // check to make sure that the sender has blacklister permissions
    check_bool_allowance(deps.as_ref(), info, WHITELISTER_ALLOWANCES)?;

    let address = deps.api.addr_validate(&address)?;

    // update blacklisted status
    // validate that blacklisteed is a valid address
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        WHITELISTED_ADDRESSES.save(deps.storage, &address, &status)?;
    } else {
        WHITELISTED_ADDRESSES.remove(deps.storage, &address);
    }

    Ok(Response::new()
        .add_attribute("action", "whitelist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}

pub fn force_transfer(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    from_address: String,
    to_address: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Load TF denom for this contract
    let denom = DENOM.load(deps.storage)?;

    // Force transfer tokens
    let force_transfer_msg: CosmosMsg<TokenFactoryMsg> = MsgForceTransfer {
        transfer_from_address: from_address.clone(),
        transfer_to_address: to_address.clone(),
        amount: Some(Coin::new(amount.u128(), denom.clone()).into()),
        sender: env.contract.address.to_string(),
    }
    .into();

    Ok(Response::new()
        .add_attribute("action", "force_transfer")
        .add_attribute("from_address", from_address)
        .add_attribute("to_address", to_address)
        .add_message(force_transfer_msg))
}
