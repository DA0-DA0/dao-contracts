#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

const CONTRACT_NAME: &str = "crates.io:native-token-staked-balance-voting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height: _ } => {
            query_voting_power_at_height(deps, env, address)
        }
        QueryMsg::Info {} => query_info(deps),
        _ => to_binary(&"Query Not Implemented"),
    }
}

pub fn query_voting_power_at_height(deps: Deps, env: Env, address: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let delegations = deps.querier.query_all_delegations(address)?;
    let voting_power = Uint128::from(0u128);

    for delegation in delegations.into_iter() {
        voting_power.checked_add(delegation.amount.amount)?;
    }

    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse {
        power: voting_power,
        height: env.block.height,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}
