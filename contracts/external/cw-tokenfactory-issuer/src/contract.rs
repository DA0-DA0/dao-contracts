use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
};
use cw2::{get_contract_version, set_contract_version, ContractVersion};
use cw_tokenfactory_types::msg::{msg_create_denom, MsgCreateDenomResponse};

use crate::error::ContractError;
use crate::execute;
use crate::hooks;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::queries;
use crate::state::{BeforeSendHookInfo, BEFORE_SEND_HOOK_INFO, DENOM, IS_FROZEN};

// Version info for migration
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const CREATE_DENOM_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Owner is the sender of the initial InstantiateMsg
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    // BeforeSendHook features are disabled by default.
    BEFORE_SEND_HOOK_INFO.save(
        deps.storage,
        &BeforeSendHookInfo {
            advanced_features_enabled: false,
            hook_contract_address: None,
        },
    )?;
    IS_FROZEN.save(deps.storage, &false)?;

    match msg {
        InstantiateMsg::NewToken { subdenom } => {
            Ok(Response::new()
                .add_attribute("action", "instantiate")
                .add_attribute("owner", info.sender)
                .add_attribute("subdenom", subdenom.clone())
                .add_submessage(
                    // Create new denom, denom info is saved in the reply
                    SubMsg::reply_on_success(
                        msg_create_denom(env.contract.address.to_string(), subdenom),
                        CREATE_DENOM_REPLY_ID,
                    ),
                ))
        }
        InstantiateMsg::ExistingToken { denom } => {
            DENOM.save(deps.storage, &denom)?;

            Ok(Response::new()
                .add_attribute("action", "instantiate")
                .add_attribute("owner", info.sender)
                .add_attribute("denom", denom))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Executive Functions
        ExecuteMsg::Mint { to_address, amount } => {
            execute::mint(deps, env, info, to_address, amount)
        }
        ExecuteMsg::Burn {
            amount,
            from_address: address,
        } => execute::burn(deps, env, info, amount, address),
        ExecuteMsg::Deny { address, status } => execute::deny(deps, env, info, address, status),
        ExecuteMsg::Allow { address, status } => execute::allow(deps, info, address, status),
        ExecuteMsg::Freeze { status } => execute::freeze(deps, env, info, status),

        #[cfg(feature = "osmosis_tokenfactory")]
        ExecuteMsg::ForceTransfer {
            amount,
            from_address,
            to_address,
        } => execute::force_transfer(deps, env, info, amount, from_address, to_address),

        // Admin functions
        ExecuteMsg::UpdateTokenFactoryAdmin { new_admin } => {
            execute::update_tokenfactory_admin(deps, env, info, new_admin)
        }
        ExecuteMsg::UpdateOwnership(action) => {
            execute::update_contract_owner(deps, env, info, action)
        }
        ExecuteMsg::SetMinterAllowance { address, allowance } => {
            execute::set_minter(deps, info, address, allowance)
        }
        ExecuteMsg::SetBurnerAllowance { address, allowance } => {
            execute::set_burner(deps, info, address, allowance)
        }
        #[cfg(feature = "osmosis_tokenfactory")]
        ExecuteMsg::SetBeforeSendHook { cosmwasm_address } => {
            execute::set_before_send_hook(deps, env, info, cosmwasm_address)
        }
        ExecuteMsg::SetDenomMetadata { metadata } => {
            execute::set_denom_metadata(deps, env, info, metadata)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BlockBeforeSend { from, to, amount } => {
            hooks::beforesend_hook(deps, from, to, amount)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Allowlist { start_after, limit } => {
            to_json_binary(&queries::query_allowlist(deps, start_after, limit)?)
        }
        QueryMsg::BeforeSendHookInfo {} => {
            to_json_binary(&queries::query_before_send_hook_features(deps)?)
        }
        QueryMsg::BurnAllowance { address } => {
            to_json_binary(&queries::query_burn_allowance(deps, address)?)
        }
        QueryMsg::BurnAllowances { start_after, limit } => {
            to_json_binary(&queries::query_burn_allowances(deps, start_after, limit)?)
        }
        QueryMsg::Denom {} => to_json_binary(&queries::query_denom(deps)?),
        QueryMsg::Denylist { start_after, limit } => {
            to_json_binary(&queries::query_denylist(deps, start_after, limit)?)
        }
        QueryMsg::IsAllowed { address } => {
            to_json_binary(&queries::query_is_allowed(deps, address)?)
        }
        QueryMsg::IsDenied { address } => to_json_binary(&queries::query_is_denied(deps, address)?),
        QueryMsg::IsFrozen {} => to_json_binary(&queries::query_is_frozen(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&queries::query_owner(deps)?),
        QueryMsg::MintAllowance { address } => {
            to_json_binary(&queries::query_mint_allowance(deps, address)?)
        }
        QueryMsg::MintAllowances { start_after, limit } => {
            to_json_binary(&queries::query_mint_allowances(deps, start_after, limit)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let storage_version: ContractVersion = get_contract_version(deps.storage)?;

    // Only migrate if newer
    if storage_version.version.as_str() < CONTRACT_VERSION {
        // Set contract to version to latest
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CREATE_DENOM_REPLY_ID => {
            let MsgCreateDenomResponse { new_token_denom } = msg.result.try_into()?;
            DENOM.save(deps.storage, &new_token_denom)?;

            Ok(Response::new().add_attribute("denom", new_token_denom))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
