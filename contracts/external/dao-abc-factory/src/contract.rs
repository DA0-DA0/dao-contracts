#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Reply, Response,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_abc::msg::{InstantiateMsg as AbcInstantiateMsg, QueryMsg as AbcQueryMsg};
use cw_storage_plus::{Bound, Item, Map};
use cw_utils::parse_reply_instantiate_data;
use dao_interface::{token::TokenFactoryCallback, voting::Query as VotingModuleQueryMsg};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_ABC_REPLY_ID: u64 = 1;

const DAOS: Map<Addr, Empty> = Map::new("daos");
const VOTING_MODULE: Item<Addr> = Item::new("voting_module");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AbcFactory(msg) => execute_token_factory_factory(deps, env, info, msg),
    }
}

pub fn execute_token_factory_factory(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: AbcInstantiateMsg,
) -> Result<Response, ContractError> {
    // Save voting module address
    VOTING_MODULE.save(deps.storage, &info.sender)?;

    // Query for DAO
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &VotingModuleQueryMsg::Dao {})?;

    DAOS.save(deps.storage, dao, &Empty {})?;

    // Instantiate new contract, further setup is handled in the
    // SubMsg reply.
    let msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            // No admin as we want the bonding curve contract to be immutable
            admin: None,
            code_id: msg.token_issuer_code_id,
            msg: to_json_binary(&msg)?,
            funds: vec![],
            label: "cw_abc".to_string(),
        },
        INSTANTIATE_ABC_REPLY_ID,
    );

    Ok(Response::new().add_submessage(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Daos { start_after, limit } => query_daos(deps, start_after, limit),
    }
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_daos(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    to_json_binary(
        &DAOS
            .keys(
                deps.storage,
                None,
                start_after
                    .map(|s| deps.api.addr_validate(&s))
                    .transpose()?
                    .map(Bound::exclusive),
                Order::Descending,
            )
            .take(limit.unwrap_or(25) as usize)
            .collect::<StdResult<Vec<Addr>>>()?,
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_ABC_REPLY_ID => {
            // Parse issuer address from instantiate reply
            let abc_addr = parse_reply_instantiate_data(msg)?.contract_address;

            // Query for denom
            let denom = deps
                .querier
                .query_wasm_smart(abc_addr.clone(), &AbcQueryMsg::Denom {})?;

            // Query for token contract
            let token_contract: Addr = deps
                .querier
                .query_wasm_smart(abc_addr.clone(), &AbcQueryMsg::TokenContract {})?;

            // Responses for `dao-voting-token-staked` MUST include a
            // TokenFactoryCallback.
            Ok(
                Response::new().set_data(to_json_binary(&TokenFactoryCallback {
                    denom,
                    token_contract: Some(token_contract.to_string()),
                    module_instantiate_callback: None,
                })?),
            )
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
