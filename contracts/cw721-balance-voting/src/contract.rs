#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenInfo};
use crate::state::TOKEN;

const CONTRACT_NAME: &str = "crates.io:cw721-balance-voting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg.token_info {
        TokenInfo::Existing { address } => {
            let address = deps.api.addr_validate(&address)?;
            TOKEN.save(deps.storage, &address)?;
            Ok(Response::default()
                .add_attribute("action", "instantiate")
                .add_attribute("token", "existing_nft")
                .add_attribute("token_address", address))
        }
    }
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
        QueryMsg::TokenContract {} => query_token_contract(deps),
        QueryMsg::VotingPowerAtHeight { address, height: _ } => {
            query_voting_power_at_height(deps, env, address)
        }
        QueryMsg::TotalPowerAtHeight { height: _ } => query_total_power_at_height(deps, env),
        QueryMsg::Info {} => query_info(deps),
    }
}

pub fn query_token_contract(deps: Deps) -> StdResult<Binary> {
    let token = TOKEN.load(deps.storage)?;
    to_binary(&token)
}

pub fn query_voting_power_at_height(deps: Deps, env: Env, address: String) -> StdResult<Binary> {
    let token = TOKEN.load(deps.storage)?;
    let address = deps.api.addr_validate(&address)?;
    let balance: cw721::TokensResponse = deps.querier.query_wasm_smart(
        token,
        &cw721::Cw721QueryMsg::Tokens {
            owner: address.to_string(),
            start_after: None,
            limit: None,
        },
    )?;
    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse {
        power: Uint128::from(u128::try_from(balance.tokens.len()).unwrap()),
        height: env.block.height,
    })
}

pub fn query_total_power_at_height(deps: Deps, env: Env) -> StdResult<Binary> {
    let token = TOKEN.load(deps.storage)?;
    let info: cw721::NumTokensResponse = deps
        .querier
        .query_wasm_smart(token, &cw721::Cw721QueryMsg::NumTokens {})?;
    to_binary(&cw_core_interface::voting::TotalPowerAtHeightResponse {
        power: Uint128::from(info.count),
        height: env.block.height,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}
