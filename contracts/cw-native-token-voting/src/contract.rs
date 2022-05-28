#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{DAO, TOKEN_DENOM};

const CONTRACT_NAME: &str = "crates.io:cw-native-token-voting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    DAO.save(deps.storage, &info.sender)?;
    TOKEN_DENOM.save(deps.storage, &msg.token_denom)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("token_denom", msg.token_denom))
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
            query_voting_power_at_height(deps, env, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::TokenDenom {} => query_token_denom(deps),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    _height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let token_denom = TOKEN_DENOM.load(deps.storage)?;
    let delegations = deps.querier.query_all_delegations(address)?;
    let power: Uint128 = delegations
        .iter()
        .map(|a| -> Uint128 {
            if a.amount.denom == token_denom {
                a.amount.amount
            } else {
                Uint128::zero()
            }
        })
        .sum();
    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse {
        power,
        height: env.block.height,
    })
}

pub fn query_total_power_at_height(
    _deps: Deps,
    env: Env,
    _height: Option<u64>,
) -> StdResult<Binary> {
    // TODO: What do I put here?
    to_binary(&cw_core_interface::voting::TotalPowerAtHeightResponse {
        power: Uint128::zero(),
        height: env.block.height,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_binary(&dao)
}

pub fn query_token_denom(deps: Deps) -> StdResult<Binary> {
    let token_denom = TOKEN_DENOM.load(deps.storage)?;
    to_binary(&token_denom)
}
