#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version, ContractVersion};
// use cw_controllers::ClaimsResponse;
// use cw_storage_plus::Bound;
// use cw_tokenfactory_issuer::msg::{
//     DenomUnit, ExecuteMsg as IssuerExecuteMsg, InstantiateMsg as IssuerInstantiateMsg, Metadata,
// };
// use cw_utils::{
//     maybe_addr, must_pay, parse_reply_execute_data, parse_reply_instantiate_data, Duration,
// };
use dao_interface::voting::{
    TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetHooksResponse, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::state::{CONFIG, DAO, HOOKS, STAKED_BALANCES, STAKED_TOTAL};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-cosmos-staking";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Settings for query pagination
// const MAX_LIMIT: u32 = 30;
// const DEFAULT_LIMIT: u32 = 10;

// We multiply by this when calculating needed power for being active
// when using active threshold with percent
// const PRECISION_FACTOR: u128 = 10u128.pow(9);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
    }
}

pub fn execute_add_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook)?;
    Ok(Response::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook)?;
    Ok(Response::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_json_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_json_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::GetConfig {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::IsActive {} => unimplemented!(),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let address = deps.api.addr_validate(&address)?;
    let power = STAKED_BALANCES
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = STAKED_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(TotalPowerAtHeightResponse { power, height })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_json_binary(&dao)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let storage_version: ContractVersion = get_contract_version(deps.storage)?;

    // Only migrate if newer
    if storage_version.version.as_str() < CONTRACT_VERSION {
        // Set contract to version to latest
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    Ok(Response::new().add_attribute("action", "migrate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(_deps: DepsMut, _env: Env, _msg: SudoMsg) -> Result<Response, ContractError> {
    // SudoMsg::BeforeDelegationCreated {
    //     validator_address,
    //     delegator_address,
    //     shares,
    // } => last_delegation(deps, validator_address, delegator_address, shares),
    // SudoMsg::BeforeDelegationSharesModified {
    //     validator_address,
    //     delegator_address,
    //     shares,
    // } => last_delegation(deps, validator_address, delegator_address, shares),
    // SudoMsg::BeforeDelegationRemoved {
    //     validator_address,
    //     delegator_address,
    //     shares,
    // } => last_delegation(deps, validator_address, delegator_address, shares),
    unimplemented!()
}
