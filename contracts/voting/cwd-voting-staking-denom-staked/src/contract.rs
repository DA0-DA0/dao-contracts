#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cwd_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{DAO, STAKING_MODULE};

const CONTRACT_NAME: &str = "crates.io:cwd-voting-staking-denom-staked";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let validated_staking_module = deps.api.addr_validate(&msg.staking_module_address)?;

    STAKING_MODULE.save(deps.storage, &validated_staking_module)?;
    DAO.save(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("staking_module", validated_staking_module))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::StakingModule {} => query_staking_module(deps),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    // We can ignore height as we are protected by the chain's unstaking
    // duration
    let denom = deps.querier.query_bonded_denom()?;
    let delegations = deps.querier.query_all_delegations(address)?;
    let power = delegations
        .iter()
        .filter_map(|d| {
            if d.amount.denom == denom {
                Some(d.amount.amount)
            } else {
                None
            }
        })
        .reduce(|a, b| a.checked_add(b).unwrap())
        .unwrap_or_default();

    Ok(VotingPowerAtHeightResponse {
        power,
        height: height.unwrap_or(env.block.height),
    })
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let staking_module = STAKING_MODULE.load(deps.storage)?;
    let denom = deps.querier.query_bonded_denom()?;
    let power = deps.querier.query_balance(staking_module, denom)?;
    Ok(TotalPowerAtHeightResponse {
        power: power.amount,
        height: height.unwrap_or(env.block.height),
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cwd_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_binary(&dao)
}

pub fn query_staking_module(deps: Deps) -> StdResult<Binary> {
    let staking_module = STAKING_MODULE.load(deps.storage)?;
    to_binary(&staking_module)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Don't do any state migrations.
    Ok(Response::default())
}
