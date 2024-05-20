#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use dao_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{DAO, STAKED_TOTAL};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-cosmos-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DAO.save(deps.storage, &info.sender)?;
    STAKED_TOTAL.save(deps.storage, &msg.total_staked, env.block.height)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::UpdateTotalStaked { amount, height } => {
            execute_update_total_staked(deps, env, amount, height)
        }
    }
}

pub fn execute_update_total_staked(
    deps: DepsMut,
    env: Env,
    amount: Uint128,
    height: Option<u64>,
) -> Result<Response, ContractError> {
    STAKED_TOTAL.save(deps.storage, &amount, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "update_total_staked")
        .add_attribute("amount", amount)
        .add_attribute("height", height.unwrap_or(env.block.height).to_string()))
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
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    // Lie about height since we can't access historical data.
    let height = height.unwrap_or(env.block.height);
    let power = get_total_delegations(deps, address)?;

    Ok(VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    // Total staked amount is initialized to a value during contract
    // instantiation. Any block before that block returns 0.
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

fn get_total_delegations(deps: Deps, delegator: String) -> StdResult<Uint128> {
    let delegations = deps.querier.query_all_delegations(delegator)?;

    let mut amount_staked = Uint128::zero();

    // iter delegations
    delegations.iter().for_each(|delegation| {
        amount_staked += delegation.amount.amount;
    });

    Ok(amount_staked)
}
