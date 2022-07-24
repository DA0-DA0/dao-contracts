#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use osmo_bindings::OsmosisMsg;

use crate::error::ContractError;
use crate::execute;
use crate::helpers;
use crate::hooks;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg};
use crate::queries;
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-usdc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // create full denom from contract addres using OsmosisModule.full_denom copy
    let contract_address = env.contract.address;
    let full_denom = helpers::build_denom(&contract_address, &msg.subdenom)?;

    let config = Config {
        owner: info.sender.clone(),
        is_frozen: false,
        denom: full_denom.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    let create_denom_msg = OsmosisMsg::CreateDenom {
        subdenom: msg.subdenom,
    };

    // hack to make sure a testable address is now admin
    let set_hook_msg = OsmosisMsg::ChangeAdmin {
        denom: full_denom,
        new_admin_address: info.sender.to_string(),
    };

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("contract", contract_address.to_string())
        .add_message(create_denom_msg)
        .add_message(set_hook_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    match msg {
        // Executive Functions
        ExecuteMsg::Mint { to_address, amount } => {
            execute::mint(deps, env, info, to_address, amount)
        }
        ExecuteMsg::Burn { amount } => execute::burn(deps, env, info, amount),
        ExecuteMsg::Blacklist { address, status } => {
            execute::blacklist(deps, env, info, address, status)
        }
        ExecuteMsg::Freeze { status } => execute::freeze(deps, env, info, status),

        // Admin functions
        ExecuteMsg::ChangeTokenFactoryAdmin { new_admin } => {
            execute::change_tokenfactory_admin(deps, env, info, new_admin)
        }
        ExecuteMsg::ChangeContractOwner { new_owner } => {
            execute::change_contract_owner(deps, env, info, new_owner)
        }
        ExecuteMsg::SetMinter { address, allowance } => {
            execute::set_minter(deps, env, info, address, allowance)
        }
        ExecuteMsg::SetBurner { address, allowance } => {
            execute::set_burner(deps, env, info, address, allowance)
        }
        ExecuteMsg::SetBlacklister { address, status } => {
            execute::set_blacklister(deps, env, info, address, status)
        }
        ExecuteMsg::SetFreezer { address, status } => {
            execute::set_freezer(deps, env, info, address, status)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BeforeSend { from, to, amount } => hooks::beforesend_hook(deps, from, to, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsFrozen {} => to_binary(&queries::query_is_frozen(deps)?),
        QueryMsg::Denom {} => to_binary(&queries::query_denom(deps)?),
        QueryMsg::Owner {} => to_binary(&queries::query_owner(deps)?),
        QueryMsg::BurnAllowance { address } => {
            to_binary(&queries::query_burn_allowance(deps, address)?)
        }
        QueryMsg::BurnAllowances { start_after, limit } => {
            to_binary(&queries::query_burn_allowances(deps, start_after, limit)?)
        }
        QueryMsg::MintAllowance { address } => {
            to_binary(&queries::query_mint_allowance(deps, address)?)
        }
        QueryMsg::MintAllowances { start_after, limit } => {
            to_binary(&queries::query_mint_allowances(deps, start_after, limit)?)
        }
        QueryMsg::IsBlacklisted { address } => {
            to_binary(&queries::query_is_blacklisted(deps, address)?)
        }
        QueryMsg::Blacklist { start_after, limit } => {
            to_binary(&queries::query_blacklist(deps, start_after, limit)?)
        }
        QueryMsg::IsBlacklister { address } => {
            to_binary(&queries::query_is_blacklister(deps, address)?)
        }
        QueryMsg::BlacklisterAllowances { start_after, limit } => {
            to_binary(&queries::query_blacklisters(deps, start_after, limit)?)
        }
        QueryMsg::IsFreezer { address } => {
            to_binary(&queries::query_freezer_allowance(deps, address)?)
        }
        QueryMsg::FreezerAllowances { start_after, limit } => to_binary(
            &queries::query_freezer_allowances(deps, start_after, limit)?,
        ),
    }
}
