#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg,
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
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        dao: info.sender.clone(),
    };
    let empty: Vec<Authorization> = vec![];
    CONFIG.save(deps.storage, &config)?;
    AUTHORIZATIONS.save(deps.storage, &info.sender, &empty)?;
    let proposal_module_msg = msg
        .proposal_module_instantiate_info
        .into_wasm_msg(env.contract.address.clone());
    let proposal_module_msg: SubMsg<Empty> = SubMsg::new(proposal_module_msg);

    Ok(Response::default()
        .add_attribute("action", "instantiate")
        .add_submessage(proposal_module_msg))
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Authorize { msgs, sender } => authorize_messages(deps, env, msgs, sender),
        QueryMsg::GetAuthorizations { .. } => {
            unimplemented!()
        }
    }
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
    println!("Auths: {:?}", auths);
    if auths.is_empty() {
        // If there aren't any authorizations, we consider the auth as not-configured and allow all
        // messages
        return to_binary(&IsAuthorizedResponse { authorized: true });
    }

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
    to_binary(&IsAuthorizedResponse { authorized })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    unimplemented!();
}
