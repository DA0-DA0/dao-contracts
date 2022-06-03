#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_proposal_single::msg::{DepositInfo, ExecuteMsg, QueryMsg};
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::{ExecuteAuthMsg, InstantiateMsg, IsAuthorizedResponse, QueryAuthMsg},
    state::{Authorization, Config, AUTHORIZATIONS, CONFIG, PROPOSAL_MODULE},
};

const CONTRACT_NAME: &str = "crates.io:cw-auth-middleware";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const PROPOSAL_MODULE_INSTANTIATE_REPLY_ID: u64 = 1;

use colored::*;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    println!("{}: {}", "DAO?".red(), info.sender.clone());
    let config = Config {
        dao: info.sender.clone(),
    };
    let empty: Vec<Authorization> = vec![];
    CONFIG.save(deps.storage, &config)?;
    AUTHORIZATIONS.save(deps.storage, &info.sender, &empty)?;
    let proposal_module_msg = msg
        .proposal_module_instantiate_info
        .into_wasm_msg(info.sender.clone()); // The admin of the proxied proposal is not this contract, but the dao.
    let proposal_module_msg: SubMsg<Empty> =
        SubMsg::reply_always(proposal_module_msg, PROPOSAL_MODULE_INSTANTIATE_REPLY_ID);

    Ok(Response::default()
        .add_attribute("action", "instantiate")
        .add_submessage(proposal_module_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ExecuteAuthMsg>,
) -> Result<Response, ContractError> {
    println!("{}", "EXECUTE BASE".blue());
    match msg {
        ExecuteMsg::Custom(auth_msg) => execute_auth_management(deps, auth_msg, info),
        base_msg => execute_proxy_contract(deps, base_msg, info),
    }
}

pub fn execute_auth_management(
    _deps: DepsMut,
    _msg: ExecuteAuthMsg,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    println!("{}", "EXECUTE MANAGEMENT".blue());
    Err(ContractError::InvalidMessageError {})
}

pub fn execute_proxy_contract(
    deps: DepsMut,
    msg: ExecuteMsg<ExecuteAuthMsg>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    println!("{} {:?}", "EXECUTE PROXY".blue(), msg);
    let proposal_addr = PROPOSAL_MODULE.load(deps.storage)?;

    let submsg = WasmMsg::Execute {
        contract_addr: proposal_addr.to_string(),
        msg: to_binary(&msg).unwrap(),
        funds: info.funds,
    };

    Ok(Response::default()
        .add_attribute("action", "execute_proxy_proposal")
        .add_message(submsg))
}

pub fn execute_add_authorization(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // ToDo: Who can add and remove auths?
    if config.dao != info.sender {
        // Only DAO can add authorizations
        return Err(ContractError::Unauthorized {
            reason: Some("Sender can't add authorization.".to_string()),
        });
    }

    // ToDo: Verify that this is an auth?
    let validated_address = deps.api.addr_validate(&address)?;
    AUTHORIZATIONS.update(
        deps.storage,
        &config.dao,
        |auths| -> Result<Vec<Authorization>, ContractError> {
            let new_auth = Authorization {
                name: "test".to_string(),
                contract: validated_address,
            };
            match auths {
                Some(mut l) => {
                    l.push(new_auth);
                    Ok(l)
                }
                None => Ok(vec![new_auth]),
            }
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "add_authorizations")
        .add_attribute("address", address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg<QueryAuthMsg>) -> StdResult<Binary> {
    println!("{} {:?}", "QUERYING BASE".yellow(), msg);
    match msg {
        QueryMsg::Custom(QueryAuthMsg::Authorize { msgs, sender }) => {
            authorize_messages(deps, env, msgs, sender)
        }
        QueryMsg::Custom(QueryAuthMsg::GetAuthorizations { .. }) => {
            unimplemented!()
        }
        base_msg => query_proxy(deps, base_msg),
    }
}

fn query_proxy(deps: Deps, msg: QueryMsg<QueryAuthMsg>) -> StdResult<Binary> {
    let proposal_addr = PROPOSAL_MODULE.load(deps.storage)?;
    println!("{}", "QUERYING INTERNAL".yellow());
    deps.querier.query_wasm_smart(proposal_addr, &msg)
}

fn authorize_messages(
    deps: Deps,
    _env: Env,
    msgs: Vec<CosmosMsg<Empty>>,
    sender: String,
) -> StdResult<Binary> {
    // This checks all the registered authorizations
    let config = CONFIG.load(deps.storage)?;
    let auths = AUTHORIZATIONS.load(deps.storage, &config.dao)?;
    println!("{} Auths: {:?}", "".yellow(), auths);
    if auths.is_empty() {
        // If there aren't any authorizations, we consider the auth as not-configured and allow all
        // messages
        return to_binary(&IsAuthorizedResponse { authorized: true });
    }

    let authorized = auths.into_iter().all(|a| {
        deps.querier
            .query_wasm_smart(
                a.contract.clone(),
                &QueryMsg::Custom(QueryAuthMsg::Authorize {
                    msgs: msgs.clone(),
                    sender: sender.clone(),
                }),
            )
            .unwrap_or(IsAuthorizedResponse { authorized: false })
            .authorized
    });
    to_binary(&IsAuthorizedResponse { authorized })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        PROPOSAL_MODULE_INSTANTIATE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let proposal_module_addr = deps.api.addr_validate(&res.contract_address)?;
            let current = PROPOSAL_MODULE.may_load(deps.storage)?;
            println!(
                "{}: {:?}",
                "PROPOSAL (PROXIED) ADDR".blue(),
                proposal_module_addr
            );
            // Make sure a bug in instantiation isn't causing us to
            // make more than one proposal module.
            if current.is_some() {
                return Err(ContractError::MultipleParents {});
            }

            PROPOSAL_MODULE.save(deps.storage, &proposal_module_addr)?;

            let own_config = CONFIG.load(deps.storage)?;
            let mut proposal_config: cw_proposal_single::state::Config = deps
                .querier
                .query_wasm_smart(proposal_module_addr.clone(), &QueryMsg::Config::<Empty> {})?;
            proposal_config.dao = own_config.dao;
            println!("{}{:?}", "NEW_CONFIG:".red(), proposal_config);

            // Now that I own the proposal. I update its config so that it knows who the real dao is
            let deposit = if let Some(deposit) = proposal_config.deposit_info {
                Some(DepositInfo::from_checked(deposit))
            } else {
                None
            };

            // Can we do this in a cleaner way?
            let proposal_update_msg = WasmMsg::Execute {
                contract_addr: proposal_module_addr.to_string(),
                msg: to_binary(&ExecuteMsg::UpdateConfig::<Empty> {
                    threshold: proposal_config.threshold,
                    max_voting_period: proposal_config.max_voting_period,
                    only_members_execute: proposal_config.only_members_execute,
                    allow_revoting: proposal_config.allow_revoting,
                    dao: proposal_config.dao.to_string(),
                    deposit_info: deposit,
                })
                .unwrap(),
                funds: vec![],
            };

            Ok(Response::default()
                .add_attribute("proposal_module", proposal_module_addr)
                .add_submessage(SubMsg::new(proposal_update_msg)))
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}
