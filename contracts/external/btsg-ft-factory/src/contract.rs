#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsg,
};

use cw2::set_contract_version;
use dao_interface::token::{InitialBalance, TokenFactoryCallback};

use crate::bitsong::{Coin, MsgIssue, MsgMint, MsgSetMinter};
use crate::error::ContractError;
use crate::msg::{CreatingFanToken, ExecuteMsg, InstantiateMsg, MigrateMsg, NewFanToken, QueryMsg};
use crate::state::CREATING_FAN_TOKEN;

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
        ExecuteMsg::Issue(issue_info) => execute_issue(deps, env, info, issue_info),
    }
}

pub fn execute_issue(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: NewFanToken,
) -> Result<Response, ContractError> {
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &dao_interface::voting::Query::Dao {})?;

    CREATING_FAN_TOKEN.save(
        deps.storage,
        &CreatingFanToken {
            token: token.clone(),
            dao: dao.clone(),
        },
    )?;

    let msg = SubMsg::reply_on_success(
        MsgIssue {
            symbol: token.symbol,
            name: token.name,
            max_supply: token.max_supply.to_string(),
            authority: dao.to_string(),
            // will be set to DAO in reply once initial balances are minted
            minter: env.contract.address.to_string(),
            uri: token.uri,
        },
        ISSUE_REPLY_ID,
    );

    Ok(Response::default()
        .add_attribute("action", "issue")
        .add_submessage(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err("no queries"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        ISSUE_REPLY_ID => {
            // must load fan token info from execution
            let CreatingFanToken { token, dao } = CREATING_FAN_TOKEN.load(deps.storage)?;

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

            // mgs to be executed to finalize setup
            let mut msgs: Vec<CosmosMsg> = vec![];

            // mint tokens for initial balances
            token
                .initial_balances
                .iter()
                .for_each(|b: &InitialBalance| {
                    msgs.push(
                        MsgMint {
                            recipient: b.address.clone(),
                            coin: Some(Coin {
                                amount: b.amount.to_string(),
                                denom: denom.clone(),
                            }),
                            minter: env.contract.address.to_string(),
                        }
                        .into(),
                    );
                });

            // add initial DAO balance to initial_balances if nonzero
            if let Some(initial_dao_balance) = token.initial_dao_balance {
                if !initial_dao_balance.is_zero() {
                    msgs.push(
                        MsgMint {
                            recipient: dao.to_string(),
                            coin: Some(Coin {
                                amount: initial_dao_balance.to_string(),
                                denom: denom.clone(),
                            }),
                            minter: env.contract.address.to_string(),
                        }
                        .into(),
                    );
                }
            }

            // set minter to DAO
            msgs.push(
                MsgSetMinter {
                    denom: denom.clone(),
                    old_minter: env.contract.address.to_string(),
                    new_minter: dao.to_string(),
                }
                .into(),
            );

            // create reply data for dao-voting-token-staked
            let data = to_json_binary(&TokenFactoryCallback {
                denom: denom.clone(),
                token_contract: None,
                module_instantiate_callback: None,
            })?;

            // remove since we don't need it anymore
            CREATING_FAN_TOKEN.remove(deps.storage);

            Ok(Response::default()
                .add_messages(msgs)
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
