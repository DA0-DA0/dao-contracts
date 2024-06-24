use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use dao_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::DAO;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-cosmos-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DAO.save(deps.storage, &info.sender)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Err(ContractError::NoExecute {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, .. } => {
            to_json_binary(&query_voting_power_at_height(deps, env, address)?)
        }
        QueryMsg::TotalPowerAtHeight { .. } => {
            to_json_binary(&query_total_power_at_height(deps, env)?)
        }
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
) -> StdResult<VotingPowerAtHeightResponse> {
    let power = get_delegator_total(deps, address)?;

    Ok(VotingPowerAtHeightResponse {
        power,
        // always return the latest block height since we can't access
        // historical data
        height: env.block.height,
    })
}

pub fn query_total_power_at_height(deps: Deps, env: Env) -> StdResult<TotalPowerAtHeightResponse> {
    let power = get_total_delegated(deps)?;

    Ok(TotalPowerAtHeightResponse {
        power,
        // always return the latest block height since we can't access
        // historical data
        height: env.block.height,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_json_binary(&dao)
}

fn get_delegator_total(deps: Deps, delegator: String) -> StdResult<Uint128> {
    let delegations = deps.querier.query_all_delegations(delegator)?;

    let mut amount_staked = Uint128::zero();

    // iter delegations
    delegations.iter().for_each(|delegation| {
        amount_staked += delegation.amount.amount;
    });

    Ok(amount_staked)
}

fn get_total_delegated(deps: Deps) -> StdResult<Uint128> {
    let pool = osmosis_std::types::cosmos::staking::v1beta1::QueryPoolRequest {}
        .query(&deps.querier)?
        .pool
        .unwrap();

    Ok(Uint128::from_str(pool.bonded_tokens.as_ref()).unwrap())
}
