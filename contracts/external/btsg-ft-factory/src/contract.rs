#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
    SubMsg,
};

use cw2::set_contract_version;
use dao_interface::token::TokenFactoryCallback;

use crate::bitsong::MsgIssue;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub(crate) const CONTRACT_NAME: &str = "crates.io:btsg-ft-factory";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ISSUE_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("creator", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Issue {
            symbol,
            name,
            max_supply,
            authority,
            minter,
            uri,
        } => execute_issue(
            deps,
            env,
            info,
            MsgIssue {
                symbol,
                name,
                max_supply: max_supply.to_string(),
                authority,
                minter,
                uri,
            },
        ),
    }
}

pub fn execute_issue(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: MsgIssue,
) -> Result<Response, ContractError> {
    let msg = SubMsg::reply_on_success(msg, ISSUE_REPLY_ID);
    Ok(Response::default()
        .add_attribute("action", "issue")
        .add_submessage(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err("no queries"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        ISSUE_REPLY_ID => {
            // find eventissue->denom event attribute
            let denom = msg
                .result
                .into_result()
                .unwrap()
                .events
                .into_iter()
                .find(|e| e.ty == "bitsong.fantoken.v1beta1.EventIssue")
                .unwrap()
                .attributes
                .into_iter()
                .find(|a| a.key == "denom")
                .unwrap()
                .value;

            // create reply data for dao-voting-token-staked
            let data = to_json_binary(&TokenFactoryCallback {
                denom: denom.clone(),
                token_contract: None,
                module_instantiate_callback: None,
            })?;

            Ok(Response::default()
                .set_data(data)
                .add_attribute("fantoken_denom", denom))
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
