use cosmwasm_std::{coins, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use osmosis_std::types::cosmos::bank::v1beta1::Metadata;
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgForceTransfer, MsgSetBeforeSendHook, MsgSetDenomMetadata,
};
use token_bindings::TokenFactoryMsg;

use crate::error::ContractError;
use crate::helpers::{check_before_send_hook_features_enabled, check_is_not_frozen};
use crate::state::{
    BeforeSendHookInfo, ALLOWLIST, BEFORE_SEND_HOOK_INFO, BURNER_ALLOWANCES, DENOM, DENYLIST,
    IS_FROZEN, MINTER_ALLOWANCES,
};

/// Mints new tokens. To mint new tokens, the address calling this method must
/// have an allowance of tokens to mint. This allowance is set by the contract through
/// the `ExecuteMsg::SetMinter { .. }` message.
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

    // Check token is not frozen, or if from or to address is on allowlist
    check_is_not_frozen(deps.as_ref(), info.sender.as_str(), &to_address, &denom)?;

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

/// Burns tokens. To burn tokens, the address calling this method must
/// have an allowance of tokens to burn and the tokens to burn must belong
/// to the `cw_tokenfactory_issuer` contract itself. The allowance is set by
/// the contract through the `ExecuteMsg::SetBurner { .. }` message, and funds
/// to be burnt must be sent to this contract prior to burning.
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

/// Updates the contract owner, must be the current contract owner to call
/// this method.
pub fn update_contract_owner(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // cw-ownable performs all validation and ownership checks for us
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

/// Updates the Token Factory token admin. To set no admin, specify the `new_admin`
/// argument to be either a null address or the address of the Cosmos SDK bank module
/// for the chain.
///
/// Must be the contract owner to call this method.
pub fn update_tokenfactory_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change tokenfactory admin
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

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

/// Sets metadata related to the Token Factory denom.
///
/// Must be the contract owner to call this method.
pub fn set_denom_metadata(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    metadata: Metadata,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to set denom metadata
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "set_denom_metadata")
        .add_message(MsgSetDenomMetadata {
            sender: env.contract.address.to_string(),
            metadata: Some(metadata),
        }))
}

/// Calls `MsgSetBeforeSendHook` and enables BeforeSendHook related features.
/// Takes a `cosmwasm_address` argument which is the address of the contract enforcing
/// the hook. Normally this will be the cw_tokenfactory_issuer contract address, but could
/// be a 3rd party address for more advanced use cases.
///
/// As not all chains support the `BeforeSendHook` in the bank module, this
/// is intended to be called should chains add this feature at a later date.
///
/// Must be the contract owner to call this method.
pub fn set_before_send_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cosmwasm_address: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // The `cosmwasm_address` can be an empty string if setting the value to nil to
    // disable the hook. If an empty string, we disable before send hook features.
    // Otherwise, we validate the `cosmwasm_address` enable before send hook features.
    if cosmwasm_address.is_empty() {
        // Disable BeforeSendHook features
        BEFORE_SEND_HOOK_INFO.save(
            deps.storage,
            &BeforeSendHookInfo {
                advanced_features_enabled: false,
                hook_contract_address: None,
            },
        )?;
    } else {
        // Validate that address is a valid address
        deps.api.addr_validate(&cosmwasm_address)?;

        // If the `cosmwasm_address` is not the same as the cw_tokenfactory_issuer contract
        // BeforeSendHook features are disabled.
        let mut advanced_features_enabled = true;
        if cosmwasm_address != env.contract.address {
            advanced_features_enabled = false;
        }

        // Save the BeforeSendHookInfo
        BEFORE_SEND_HOOK_INFO.save(
            deps.storage,
            &BeforeSendHookInfo {
                advanced_features_enabled,
                hook_contract_address: Some(cosmwasm_address.clone()),
            },
        )?;
    }

    // Load the Token Factory denom
    let denom = DENOM.load(deps.storage)?;

    // SetBeforeSendHook to this contract.
    // This will trigger sudo endpoint before any bank send,
    // which makes denylisting / freezing possible.
    let msg_set_beforesend_hook: CosmosMsg<TokenFactoryMsg> = MsgSetBeforeSendHook {
        sender: env.contract.address.to_string(),
        denom,
        cosmwasm_address,
    }
    .into();

    Ok(Response::new()
        .add_attribute("action", "set_before_send_hook")
        .add_message(msg_set_beforesend_hook))
}

/// Specifies and sets a burn allowance to allow for the burning of tokens.
/// To remove previously granted burn allowances, set this to zero.
///
/// Must be the contract owner to call this method.
pub fn set_burner(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to set burner allowance
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

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

/// Specifies and sets a burn allowance to allow for the minting of tokens.
/// To remove previously granted mint allowances, set this to zero.
///
/// Must be the contract owner to call this method.
pub fn set_minter(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    allowance: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to set minter allowance
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

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

/// Freezes / unfreezes token transfers, meaning that address will not be
/// able to send tokens until the token is unfrozen. This feature is dependent
/// on the BeforeSendHook.
///
/// This feature works in conjunction with this contract's allowlist. For example,
/// a DAO may wish to prevent its token from being liquid during its bootstrapping
/// phase. It may wish to add its staking contract to the allowlist to allow users
/// to stake their tokens (thus users would be able to transfer to the staking
/// contract), or add an airdrop contract to the allowlist so users can claim
/// their tokens (but not yet trade them).
///
/// This issuer contract itself is added to the allowlist when freezing, to allow
/// for minting of tokens (if minters with allowances are also on the allowlist).
///
/// Must be the contract owner to call this method.
pub fn freeze(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to call this method
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Update config frozen status
    // NOTE: Does not check if new status is same as old status
    IS_FROZEN.save(deps.storage, &status)?;

    // Add the issue contract itself to the Allowlist, or remove
    // if unfreezing to save storage.
    if status {
        ALLOWLIST.save(deps.storage, &env.contract.address, &status)?;
    } else {
        ALLOWLIST.remove(deps.storage, &env.contract.address);
    }

    Ok(Response::new()
        .add_attribute("action", "freeze")
        .add_attribute("status", status.to_string()))
}

/// Adds or removes an address from the denylist, meaning they will not
/// be able to transfer their tokens. This feature is dependent on
/// the BeforeSendHook.
///
/// Must be the contract owner to call this method.
pub fn deny(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to call this method
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

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

/// Relevant only when the token is frozen. Addresses on the allowlist can
/// transfer tokens as well as have tokens sent to them. This feature is
/// dependent on the BeforeSendHook.
///
/// See the `freeze` method for more information.
///
/// Must be the contract owner to call this method.
pub fn allow(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    check_before_send_hook_features_enabled(deps.as_ref())?;

    // Only allow current contract owner to call this method
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

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

/// Force transfers tokens from one account to another. To disable this,
/// DAOs will need to renounce Token Factory admin by setting the token
/// admin to be a null address or the address of the bank module.
///
/// Must be the contract owner to call this method.
pub fn force_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    from_address: String,
    to_address: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Only allow current contract owner to change owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

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
