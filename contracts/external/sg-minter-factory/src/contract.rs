#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Item;
use cw_utils::parse_reply_execute_data;
use dao_interface::voting::Query as VotingModuleQueryMsg;
use vending_factory::msg::VendingMinterCreateMsg;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_STARGAZE_MINTER_REPLY_ID: u64 = 1;

const DAO: Item<Addr> = Item::new("dao");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
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
        ExecuteMsg::StargazeBaseMinterFactory(msg) => {
            execute_stargaze_base_minter_factory(deps, env, info, msg)
        }
    }
}

/// Example Stargaze factory.
pub fn execute_stargaze_base_minter_factory(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: WasmMsg,
) -> Result<Response, ContractError> {
    // Query for DAO
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &VotingModuleQueryMsg::Dao {})?;

    DAO.save(deps.storage, &dao)?;

    // Parse msg, only an execute message is valid
    match msg {
        WasmMsg::Execute {
            contract_addr,
            msg: create_msg,
            funds,
        } => {
            // TODO no match? Doens't really make sense here
            // Match Stargaze msg
            match from_binary::<VendingMinterCreateMsg>(&create_msg)? {
                VendingMinterCreateMsg {
                    init_msg,
                    collection_params,
                } => {
                    // Replace the Stargaze info to set the DAO address
                    let mut params = collection_params;
                    params.info.creator = dao.to_string();

                    // TODO replace royalties with DAO address

                    // This creates a vending-minter contract and a sg721 contract
                    // in submsg reply, parse the response and save the contract address
                    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
                        WasmMsg::Execute {
                            contract_addr,
                            msg: to_binary(&VendingMinterCreateMsg {
                                init_msg,
                                collection_params: params,
                            })?,
                            funds,
                        },
                        INSTANTIATE_STARGAZE_MINTER_REPLY_ID,
                    )))
                }
                // TODO better error
                _ => Err(ContractError::UnsupportedFactoryMsg {}),
            }
        }
        _ => Err(ContractError::UnsupportedFactoryMsg {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => query_info(deps),
    }
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&dao_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_STARGAZE_MINTER_REPLY_ID => {
            // TODO get events
            let res = parse_reply_execute_data(msg)?;
            println!("{:?}", res);

            // TODO filter through events and find sg721_address
            // set-data in response so that the voting module can be aware of this address
            // and verify it's an NFT contract.

            unimplemented!()
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
