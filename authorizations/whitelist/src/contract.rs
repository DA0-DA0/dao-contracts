#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, to_binary};
use cw2::set_contract_version;
use cw_auth_manager::msg::{IsAuthorizedResponse, QueryMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, CONFIG};

const CONTRACT_NAME: &str = "crates.io:whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let empty: Vec<Addr> = vec![];
    let config = Config { allowed_senders: empty };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Allow { addr } => {
            CONFIG.update(deps.storage, |mut config| -> Result<Config, ContractError>{
                if !config.allowed_senders.contains(&addr){
                    config.allowed_senders.push(addr);
                }
                Ok(config)
            }).unwrap();
            Ok(Response::default().add_attribute("action", "allow"))
        },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Authorize { msgs, sender } => authorize_messages(deps, env, msgs, sender),
        _ => unimplemented!()
    }
}

fn authorize_messages(deps: Deps, _env: Env, _msgs: Vec<CosmosMsg<Empty>>, sender: Addr) -> StdResult<Binary> {
    // This checks all the registered authorizations
    let config = CONFIG.load(deps.storage)?;
    to_binary(&IsAuthorizedResponse{ authorized: config.allowed_senders.contains(&sender) })
}


#[cfg(test)]
mod tests {}
