#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenInfo};
use crate::state::{DAO, TOKEN};

const CONTRACT_NAME: &str = "crates.io:cw20-balance-voting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DAO.save(deps.storage, &info.sender)?;

    match msg.token_info {
        TokenInfo::Existing { address } => {
            let address = deps.api.addr_validate(&address)?;
            TOKEN.save(deps.storage, &address)?;
            Ok(Response::default()
                .add_attribute("action", "instantiate")
                .add_attribute("token", "existing_token")
                .add_attribute("token_address", address))
        }
        TokenInfo::New {
            code_id,
            label,
            name,
            symbol,
            decimals,
            initial_balances,
            marketing,
        } => {
            let initial_supply = initial_balances
                .iter()
                .fold(Uint128::zero(), |p, n| p + n.amount);
            if initial_supply.is_zero() {
                return Err(ContractError::InitialBalancesError {});
            }

            let msg = WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id,
                msg: to_json_binary(&cw20_base::msg::InstantiateMsg {
                    name,
                    symbol,
                    decimals,
                    initial_balances,
                    mint: Some(cw20::MinterResponse {
                        minter: info.sender.to_string(),
                        cap: None,
                    }),
                    marketing,
                })?,
                funds: vec![],
                label,
            };
            let msg = SubMsg::reply_on_success(msg, INSTANTIATE_TOKEN_REPLY_ID);

            Ok(Response::default()
                .add_attribute("action", "instantiate")
                .add_attribute("token", "new_token")
                .add_submessage(msg))
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
        QueryMsg::Dao {} => query_dao(deps),
    }
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_json_binary(&dao)
}

pub fn query_token_contract(deps: Deps) -> StdResult<Binary> {
    let token = TOKEN.load(deps.storage)?;
    to_json_binary(&token)
}

pub fn query_voting_power_at_height(deps: Deps, env: Env, address: String) -> StdResult<Binary> {
    let token = TOKEN.load(deps.storage)?;
    let address = deps.api.addr_validate(&address)?;
    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        token,
        &cw20::Cw20QueryMsg::Balance {
            address: address.to_string(),
        },
    )?;
    to_json_binary(&dao_interface::voting::VotingPowerAtHeightResponse {
        power: balance.balance,
        height: env.block.height,
    })
}

pub fn query_total_power_at_height(deps: Deps, env: Env) -> StdResult<Binary> {
    let token = TOKEN.load(deps.storage)?;
    let info: cw20::TokenInfoResponse = deps
        .querier
        .query_wasm_smart(token, &cw20::Cw20QueryMsg::TokenInfo {})?;
    to_json_binary(&dao_interface::voting::TotalPowerAtHeightResponse {
        power: info.total_supply,
        height: env.block.height,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_TOKEN_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    let token = TOKEN.may_load(deps.storage)?;
                    if token.is_some() {
                        return Err(ContractError::DuplicateToken {});
                    }
                    let token = deps.api.addr_validate(&res.contract_address)?;
                    TOKEN.save(deps.storage, &token)?;
                    Ok(Response::default().add_attribute("token_address", token))
                }
                Err(_) => Err(ContractError::TokenInstantiateError {}),
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
