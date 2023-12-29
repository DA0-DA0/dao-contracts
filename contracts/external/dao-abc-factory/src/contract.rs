#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order, Reply,
    Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_abc::msg::{
    DenomResponse, ExecuteMsg as AbcExecuteMsg, InstantiateMsg as AbcInstantiateMsg,
    QueryMsg as AbcQueryMsg,
};
use cw_storage_plus::{Bound, Item, Map};
use cw_utils::parse_reply_instantiate_data;
use dao_interface::{
    state::ModuleInstantiateCallback, token::TokenFactoryCallback,
    voting::Query as VotingModuleQueryMsg,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_ABC_REPLY_ID: u64 = 1;

const DAOS: Map<Addr, Empty> = Map::new("daos");
const CURRENT_DAO: Item<Addr> = Item::new("current_dao");
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
        ExecuteMsg::AbcFactory {
            code_id,
            instantiate_msg,
        } => execute_token_factory_factory(deps, env, info, code_id, instantiate_msg),
    }
}

pub fn execute_token_factory_factory(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code_id: u64,
    msg: AbcInstantiateMsg,
) -> Result<Response, ContractError> {
    // Save voting module address
    VOTING_MODULE.save(deps.storage, &info.sender)?;

    // Query for DAO
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &VotingModuleQueryMsg::Dao {})?;

    DAOS.save(deps.storage, dao.clone(), &Empty {})?;
    CURRENT_DAO.save(deps.storage, &dao)?;

    // Instantiate new contract, further setup is handled in the
    // SubMsg reply.
    let msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            // No admin as we want the bonding curve contract to be immutable
            admin: None,
            code_id,
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
            // Load DAO
            let dao = CURRENT_DAO.load(deps.storage)?;

            // Parse issuer address from instantiate reply
            let abc_addr = parse_reply_instantiate_data(msg)?.contract_address;

            // Query for denom
            let denom: DenomResponse = deps
                .querier
                .query_wasm_smart(abc_addr.clone(), &AbcQueryMsg::Denom {})?;

            // Query for token contract
            let token_contract: Addr = deps
                .querier
                .query_wasm_smart(abc_addr.clone(), &AbcQueryMsg::TokenContract {})?;

            // Update the owner to be the DAO
            let msg = WasmMsg::Execute {
                contract_addr: abc_addr.clone(),
                msg: to_json_binary(&AbcExecuteMsg::UpdateOwnership(
                    cw_ownable::Action::TransferOwnership {
                        new_owner: dao.to_string(),
                        expiry: None,
                    },
                ))?,
                funds: vec![],
            };

            // DAO must accept ownership transfer. Here we include a
            // ModuleInstantiateCallback message that will be called by the
            // dao-dao-core contract when voting module instantiation is
            // complete.
            let callback = ModuleInstantiateCallback {
                msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: abc_addr.clone(),
                    msg: to_json_binary(&AbcExecuteMsg::UpdateOwnership(
                        cw_ownable::Action::AcceptOwnership {},
                    ))?,
                    funds: vec![],
                })],
            };

            // Responses for `dao-voting-token-staked` MUST include a
            // TokenFactoryCallback.
            Ok(Response::new()
                .add_message(msg)
                .set_data(to_json_binary(&TokenFactoryCallback {
                    denom: denom.denom,
                    token_contract: Some(token_contract.to_string()),
                    module_instantiate_callback: Some(callback),
                })?))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
