use cosmwasm_std::{coins, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use osmosis_std::types::cosmos::bank::v1beta1::Metadata;
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgForceTransfer, MsgSetBeforeSendHook, MsgSetDenomMetadata,
};
use token_bindings::TokenFactoryMsg;

use crate::error::ContractError;
use crate::helpers::{check_before_send_hook_features_enabled, check_is_contract_owner};
use crate::state::{
    ALLOWLIST, BEFORE_SEND_HOOK_FEATURES_ENABLED, BURNER_ALLOWANCES, DENOM, DENYLIST, IS_FROZEN,
    MINTER_ALLOWANCES, OWNER,
};

pub fn mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_address: String,
    amount: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Validate that to_address is a valid address
    deps.api.addr_validate(&to_address)?;

    // Don't allow minting of 0 coins
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    // Decrease minter allowance
    let allowance = MINTER_ALLOWANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(Uint128::zero);

    // If minter allowance goes negative, throw error
    let updated_allowance = allowance
        .checked_sub(amount)
        .map_err(|_| ContractError::not_enough_mint_allowance(amount, allowance))?;

    // If minter allowance goes 0, remove from storage
    if updated_allowance.is_zero() {
        MINTER_ALLOWANCES.remove(deps.storage, &info.sender);
    } else {
        MINTER_ALLOWANCES.save(deps.storage, &info.sender, &updated_allowance)?;
    }

    // Get token denom from contract
    let denom = DENOM.load(deps.storage)?;

    // Create tokenfactory MsgMint which mints coins to the contract address
    let mint_tokens_msg = TokenFactoryMsg::mint_contract_tokens(
        denom.clone(),
        amount,
        env.contract.address.into_string(),
    );

    // Send newly minted coins from contract to designated recipient
    let send_tokens_msg = BankMsg::Send {
        to_address: to_address.clone(),
        amount: coins(amount.u128(), denom),
    };

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
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Don't allow burning of 0 coins
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    // Decrease burner allowance
    let allowance = BURNER_ALLOWANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(Uint128::zero);

    // If burner allowance goes negative, throw error
    let updated_allowance = allowance
        .checked_sub(amount)
        .map_err(|_| ContractError::not_enough_burn_allowance(amount, allowance))?;

    // If burner allowance goes 0, remove from storage
    if updated_allowance.is_zero() {
        BURNER_ALLOWANCES.remove(deps.storage, &info.sender);
    } else {
        BURNER_ALLOWANCES.save(deps.storage, &info.sender, &updated_allowance)?;
    }

    // Get token denom from contract config
    let denom = DENOM.load(deps.storage)?;

    // Create tokenfactory MsgBurn which burns coins from the contract address
    // NOTE: this requires the contract to own the tokens already
    let burn_from_address = deps.api.addr_validate(&address)?;
    let burn_tokens_msg: cosmwasm_std::CosmosMsg<TokenFactoryMsg> = MsgBurn {
        sender: env.contract.address.to_string(),
        amount: Some(Coin::new(amount.u128(), denom).into()),
        burn_from_address: burn_from_address.to_string(),
    }
    .into();

    Ok(Response::new()
        .add_message(burn_tokens_msg)
        .add_attribute("action", "burn")
        .add_attribute("burner", info.sender)
        .add_attribute("burn_from_address", burn_from_address.to_string())
        .add_attribute("amount", amount))
}

pub fn update_contract_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Validate that new owner is a valid address
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;

    // Update the contract owner in the contract config
    OWNER.save(deps.storage, &new_owner_addr)?;

    Ok(Response::new()
        .add_attribute("action", "update_contract_owner")
        .add_attribute("new_owner", new_owner))
}

pub fn update_tokenfactory_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change tokenfactory admin
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Validate that the new admin is a valid address
    let new_admin_addr = deps.api.addr_validate(&new_admin)?;

    // Construct tokenfactory change admin msg
    let update_admin_msg = TokenFactoryMsg::ChangeAdmin {
        denom: DENOM.load(deps.storage)?,
        new_admin_address: new_admin_addr.into(),
    };

    Ok(Response::new()
        .add_message(update_admin_msg)
        .add_attribute("action", "update_tokenfactory_admin")
        .add_attribute("new_admin", new_admin))
}

pub fn set_denom_metadata(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    metadata: Metadata,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to set denom metadata
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "set_denom_metadata")
        .add_message(MsgSetDenomMetadata {
            sender: env.contract.address.to_string(),
            metadata: Some(metadata),
        }))
}

pub fn set_before_send_hook(
    deps: DepsMut,
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

    // SetBeforeSendHook to this contract.
    // This will trigger sudo endpoint before any bank send,
    // which makes denylisting / freezing possible.
    let msg_set_beforesend_hook: CosmosMsg<TokenFactoryMsg> = MsgSetBeforeSendHook {
        sender: env.contract.address.to_string(),
        denom,
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
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to set burner allowance
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Validate that burner is a valid address
    let address = deps.api.addr_validate(&address)?;

    // Update allowance of burner, remove key from state if set to 0
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
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to set minter allowance
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Validate that minter is a valid address
    let address = deps.api.addr_validate(&address)?;

    // Update allowance of minter, remove key from state if set to 0
    if allowance.is_zero() {
        MINTER_ALLOWANCES.remove(deps.storage, &address);
    } else {
        MINTER_ALLOWANCES.save(deps.storage, &address, &allowance)?;
    }

    Ok(Response::new()
        .add_attribute("action", "set_minter")
        .add_attribute("minter", address)
        .add_attribute("allowance", allowance))
}

pub fn freeze(
    deps: DepsMut,
    info: MessageInfo,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to call this method
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    // Update config frozen status
    // NOTE: Does not check if new status is same as old status
    IS_FROZEN.save(deps.storage, &status)?;

    Ok(Response::new()
        .add_attribute("action", "freeze")
        .add_attribute("status", status.to_string()))
}

pub fn deny(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to call this method
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    let address = deps.api.addr_validate(&address)?;

    // Check this issuer contract is not denylisting itself
    if address == env.contract.address {
        return Err(ContractError::CannotDenylistSelf {});
    }

    // Update denylist status and validate that denylistee is a valid address
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        DENYLIST.save(deps.storage, &address, &status)?;
    } else {
        DENYLIST.remove(deps.storage, &address);
    }

    Ok(Response::new()
        .add_attribute("action", "denylist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}

pub fn allow(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to call this method
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    let address = deps.api.addr_validate(&address)?;

    // Update allowlist status and validate that allowlistee is a valid address
    // NOTE: Does not check if new status is same as old status
    // but if status is false, remove if exist to reduce space usage
    if status {
        ALLOWLIST.save(deps.storage, &address, &status)?;
    } else {
        ALLOWLIST.remove(deps.storage, &address);
    }

    Ok(Response::new()
        .add_attribute("action", "allowlist")
        .add_attribute("address", address)
        .add_attribute("status", status.to_string()))
}

pub fn force_transfer(
    deps: DepsMut,
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
        amount: Some(Coin::new(amount.u128(), denom).into()),
        sender: env.contract.address.to_string(),
    }
    .into();

    Ok(Response::new()
        .add_attribute("action", "force_transfer")
        .add_attribute("from_address", from_address)
        .add_attribute("to_address", to_address)
        .add_message(force_transfer_msg))
}
