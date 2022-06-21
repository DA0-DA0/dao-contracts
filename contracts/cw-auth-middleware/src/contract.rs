#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult,
};
use cw2::set_contract_version;

use crate::msg::IsAuthorizedResponse;
use crate::state::Authorization;
use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{Config, AUTHORIZATIONS, CONFIG},
};

const CONTRACT_NAME: &str = "crates.io:cw-auth-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        dao: info.sender.clone(),
    };
    let empty: Vec<Authorization> = vec![];
    CONFIG.save(deps.storage, &config)?;
    AUTHORIZATIONS.save(deps.storage, &info.sender, &empty)?;

    Ok(Response::default().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAuthorization { auth_contract } => {
            execute_add_authorization(deps, env, info, auth_contract)
        }
        ExecuteMsg::Authorize { msgs, sender } => execute_authorize(deps, msgs, sender),
    }
}

fn execute_authorize(
    deps: DepsMut,
    msgs: Vec<CosmosMsg<Empty>>,
    sender: String,
) -> Result<Response, ContractError> {
    if authorize_messages(deps.as_ref(), msgs, sender)? {
        Ok(Response::default()
            .add_attribute("action", "execute_authorize")
            .add_attribute("authorized", "true"))
    } else {
        Err(ContractError::Unauthorized { reason: None })
    }
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
                //name: "test".to_string(),
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Authorize { msgs, sender } => query_authorizations(deps, msgs, sender),
        QueryMsg::GetAuthorizations { .. } => {
            unimplemented!()
        }
    }
}

fn query_authorizations(
    deps: Deps,
    msgs: Vec<CosmosMsg<Empty>>,
    sender: String,
) -> StdResult<Binary> {
    to_binary(&IsAuthorizedResponse {
        authorized: authorize_messages(deps, msgs, sender).unwrap_or(false),
    })
}

fn authorize_messages(
    deps: Deps,
    msgs: Vec<CosmosMsg<Empty>>,
    sender: String,
) -> Result<bool, ContractError> {
    // This checks all the registered authorizations
    let config = CONFIG.load(deps.storage)?;
    let auths = AUTHORIZATIONS.load(deps.storage, &config.dao)?;

    // If there aren't any authorizations, we consider the auth as not-configured and allow all
    // messages
    let authorized = auths.into_iter().all(|a| {
        deps.querier
            .query_wasm_smart(
                a.contract.clone(),
                &QueryMsg::Authorize {
                    msgs: msgs.clone(),
                    sender: sender.clone(),
                },
            )
            .unwrap_or(IsAuthorizedResponse { authorized: false })
            .authorized
    });
    Ok(authorized)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    unimplemented!();
}
