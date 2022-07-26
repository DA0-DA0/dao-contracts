#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, wasm_execute, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Reply, Response, StdResult,
};
use cw2::set_contract_version;
use cw4::MemberListResponse;

use crate::msg::IsAuthorizedResponse;
use crate::state::Authorization;
use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{Config, AUTHORIZATIONS, CONFIG, GROUPS},
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

fn check_authorization(
    deps: Deps,
    auths: &Vec<Authorization>,
    msgs: &Vec<CosmosMsg>,
    sender: &Addr,
) -> bool {
    auths.into_iter().all(|a| {
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
    })
}

fn authorize_messages(
    deps: Deps,
    msgs: Vec<CosmosMsg>,
    sender: Addr,
) -> Result<bool, ContractError> {
    // This checks all the registered authorizations
    let config = CONFIG.load(deps.storage)?;
    let auths = AUTHORIZATIONS.load(deps.storage, &config.dao)?;

    if auths.is_empty() {
        return Ok(false);
    }

    // Check if the sender is authorized
    if check_authorization(deps, &auths, &msgs, &sender) {
        return Ok(true);
    }

    // If the sender isn't authorized, check if they belong to any groups are authorized
    let groups: StdResult<Vec<_>> = GROUPS
        .range(deps.storage, None, None, Order::Ascending)
        .filter(|r| !r.is_err())
        .filter(|g| -> bool {
            let addr = &g.as_ref().unwrap().1;
            let members: MemberListResponse = deps
                .querier
                .query_wasm_smart(
                    addr,
                    &cw4_group::msg::QueryMsg::ListMembers {
                        limit: None,
                        start_after: None,
                    },
                )
                .unwrap_or(MemberListResponse { members: vec![] });
            members.members.into_iter().any(|a| a.addr == sender)
        })
        .collect();

    let authorized = groups
        .unwrap()
        .into_iter()
        .any(|(_name, addr)| check_authorization(deps, &auths, &msgs, &addr));

    Ok(authorized)
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
        ExecuteMsg::RemoveAuthorization { auth_contract: _ } => todo!(),
        ExecuteMsg::AddGroup {
            name,
            cw4_group_contract,
        } => execute_add_group(deps, env, info, &name, &cw4_group_contract),
        ExecuteMsg::RemoveGroup { name: _ } => todo!(),
        ExecuteMsg::UpdateExecutedAuthorizationState { msgs, sender } => {
            execute_update_authorization_state(deps.as_ref(), msgs, sender, info.sender)
        }
        ExecuteMsg::Execute { msgs } => execute_execute(deps.as_ref(), msgs, info.sender),
        ExecuteMsg::ReplaceOwner { new_dao } => {
            let mut config = CONFIG.load(deps.storage)?;
            if info.sender != config.dao {
                Err(ContractError::Unauthorized { reason: None })
            } else {
                config.dao = new_dao.clone();
                CONFIG.save(deps.storage, &config)?;
                Ok(Response::default()
                    .add_attribute("action", "replace_dao")
                    .add_attribute("new_dao", new_dao))
            }
        }
    }
}

fn execute_update_authorization_state(
    deps: Deps,
    msgs: Vec<CosmosMsg>,
    sender: Addr,
    real_sender: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if sender != real_sender && real_sender != config.dao {
        return Err(ContractError::Unauthorized {
            reason: Some("Auth updates that aren't triggered by a parent contract cannot specify a sender other than the caller ".to_string()),
        });
    }

    if authorize_messages(deps, msgs.clone(), sender.clone())? {
        let auths = AUTHORIZATIONS.load(deps.storage, &config.dao)?;

        // If at least one authorization module authorized this message, we send the
        // Authorize execute message to all the authorizations so that they can update their
        // state if needed.
        let response = Response::default()
            .add_attribute("action", "execute_authorize")
            .add_attribute("authorized", "true");

        auths.iter().fold(
            Ok(response),
            |acc, auth| -> Result<Response, ContractError> {
                // TODO: Deal with the reply here. Should ignore OnError, since the validation has already been done above.
                Ok(acc?.add_message(wasm_execute(
                    auth.contract.to_string(),
                    &ExecuteMsg::UpdateExecutedAuthorizationState {
                        msgs: msgs.clone(),
                        sender: sender.clone(),
                    },
                    vec![],
                )?))
            },
        )
        // TODO: Deal with groups!!
    } else {
        Err(ContractError::Unauthorized { reason: None })
    }
}

// This method allows this contract to behave as a proposal. For this to work, the contract needs to have been instantiated by a dao.
fn execute_execute(
    deps: Deps,
    msgs: Vec<CosmosMsg>,
    sender: Addr,
) -> Result<Response, ContractError> {
    if msgs.is_empty() {
        return Err(ContractError::InvalidProposal {});
    }
    let config = CONFIG.load(deps.storage)?;

    let response = execute_update_authorization_state(
        deps.clone(),
        msgs.clone(),
        sender.clone(),
        sender.clone(),
    )?;
    let execute_msg = wasm_execute(
        config.dao.to_string(),
        &cw_core::msg::ExecuteMsg::ExecuteProposalHook { msgs },
        vec![],
    )?;

    Ok(response.add_message(execute_msg))
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

pub fn execute_add_group(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: &str,
    address: &str,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.dao != info.sender {
        // Only DAO can add groups
        return Err(ContractError::Unauthorized {
            reason: Some("Sender can't add group.".to_string()),
        });
    }

    // ToDo: Verify that this is an group?
    let validated_address = deps.api.addr_validate(&address)?;
    GROUPS.update(deps.storage, name, |addr| -> Result<Addr, ContractError> {
        if let Some(_existing) = addr {
            Err(ContractError::Unauthorized {
                reason: Some("Group already exsits".to_string()),
            })
        } else {
            Ok(validated_address)
        }
    })?;

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

fn query_authorizations(deps: Deps, msgs: Vec<CosmosMsg>, sender: Addr) -> StdResult<Binary> {
    to_binary(&IsAuthorizedResponse {
        authorized: authorize_messages(deps, msgs, sender).unwrap_or(false),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    unimplemented!();
}
