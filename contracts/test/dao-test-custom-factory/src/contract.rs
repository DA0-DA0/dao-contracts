#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721_base::InstantiateMsg as Cw721InstantiateMsg;
use cw_storage_plus::Item;
use cw_tokenfactory_issuer::msg::{
    ExecuteMsg as IssuerExecuteMsg, InstantiateMsg as IssuerInstantiateMsg,
};
use cw_utils::{one_coin, parse_reply_instantiate_data};
use dao_interface::{
    nft::NftFactoryCallback,
    token::{InitialBalance, NewTokenInfo, TokenFactoryCallback},
    voting::{ActiveThresholdQuery, Query as VotingModuleQueryMsg},
};
use dao_voting::threshold::{
    assert_valid_absolute_count_threshold, assert_valid_percentage_threshold, ActiveThreshold,
    ActiveThresholdResponse,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_ISSUER_REPLY_ID: u64 = 1;
const INSTANTIATE_NFT_REPLY_ID: u64 = 2;

const DAO: Item<Addr> = Item::new("dao");
const VOTING_MODULE: Item<Addr> = Item::new("voting_module");
const TOKEN_INFO: Item<NewTokenInfo> = Item::new("token_info");

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
        ExecuteMsg::TokenFactoryFactory(token) => {
            execute_token_factory_factory(deps, env, info, token)
        }
        ExecuteMsg::NftFactory {
            code_id,
            cw721_instantiate_msg,
        } => execute_nft_factory(deps, env, info, cw721_instantiate_msg, code_id),
    }
}

/// An example factory that instantiates a new NFT contract
/// A more realistic example would be something like a minter contract that creates
/// an NFT along with a minter contract for sales like on Stargaze.
pub fn execute_nft_factory(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cw721_instantiate_msg: Cw721InstantiateMsg,
    code_id: u64,
) -> Result<Response, ContractError> {
    // Query for DAO
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &VotingModuleQueryMsg::Dao {})?;

    // Save DAO and TOKEN_INFO for use in replies
    DAO.save(deps.storage, &dao)?;

    // Instantiate new contract, further setup is handled in the
    // SubMsg reply.
    let msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: Some(dao.to_string()),
            code_id,
            msg: to_binary(&cw721_instantiate_msg)?,
            funds: vec![],
            label: "cw_tokenfactory_issuer".to_string(),
        },
        INSTANTIATE_NFT_REPLY_ID,
    );

    Ok(Response::new().add_submessage(msg))
}

/// An example factory that instantiates a cw_tokenfactory_issuer contract
/// A more realistic example would be something like a DeFi Pool or Augmented
/// bonding curve.
pub fn execute_token_factory_factory(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token: NewTokenInfo,
) -> Result<Response, ContractError> {
    // Validate one coin was sent
    one_coin(&info)?;

    // Save voting module address
    VOTING_MODULE.save(deps.storage, &info.sender)?;

    // Query for DAO
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &VotingModuleQueryMsg::Dao {})?;

    // Save DAO and TOKEN_INFO for use in replies
    DAO.save(deps.storage, &dao)?;
    TOKEN_INFO.save(deps.storage, &token)?;

    // Instantiate new contract, further setup is handled in the
    // SubMsg reply.
    let msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: Some(dao.to_string()),
            code_id: token.token_issuer_code_id,
            msg: to_binary(&IssuerInstantiateMsg::NewToken {
                subdenom: token.subdenom,
            })?,
            funds: vec![],
            label: "cw_tokenfactory_issuer".to_string(),
        },
        INSTANTIATE_ISSUER_REPLY_ID,
    );

    Ok(Response::new().add_submessage(msg))
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
        INSTANTIATE_ISSUER_REPLY_ID => {
            // Load DAO address and TOKEN_INFO
            let dao = DAO.load(deps.storage)?;
            let token = TOKEN_INFO.load(deps.storage)?;
            let voting_module = VOTING_MODULE.load(deps.storage)?;

            // Parse issuer address from instantiate reply
            let issuer_addr = parse_reply_instantiate_data(msg)?.contract_address;

            // Format the denom
            let denom = format!("factory/{}/{}", &issuer_addr, token.subdenom);

            let initial_supply = token
                .initial_balances
                .iter()
                .fold(Uint128::zero(), |previous, new_balance| {
                    previous + new_balance.amount
                });
            let total_supply = initial_supply + token.initial_dao_balance.unwrap_or_default();

            // Here we validate the active threshold to show how validation should be done
            // in a factory contract.
            let active_threshold: ActiveThresholdResponse = deps
                .querier
                .query_wasm_smart(voting_module, &ActiveThresholdQuery::ActiveThreshold {})?;

            if let Some(threshold) = active_threshold.active_threshold {
                match threshold {
                    ActiveThreshold::Percentage { percent } => {
                        assert_valid_percentage_threshold(percent)?;
                    }
                    ActiveThreshold::AbsoluteCount { count } => {
                        assert_valid_absolute_count_threshold(count, initial_supply)?;
                    }
                }
            }

            // Msgs to be executed to finalize setup
            let mut msgs: Vec<WasmMsg> = vec![];

            // Grant an allowance to mint the initial supply
            msgs.push(WasmMsg::Execute {
                contract_addr: issuer_addr.clone(),
                msg: to_binary(&IssuerExecuteMsg::SetMinterAllowance {
                    address: env.contract.address.to_string(),
                    allowance: total_supply,
                })?,
                funds: vec![],
            });

            // Call issuer contract to mint tokens for initial balances
            token
                .initial_balances
                .iter()
                .for_each(|b: &InitialBalance| {
                    msgs.push(WasmMsg::Execute {
                        contract_addr: issuer_addr.clone(),
                        msg: to_binary(&IssuerExecuteMsg::Mint {
                            to_address: b.address.clone(),
                            amount: b.amount,
                        })
                        .unwrap_or_default(),
                        funds: vec![],
                    });
                });

            // Add initial DAO balance to initial_balances if nonzero.
            if let Some(initial_dao_balance) = token.initial_dao_balance {
                if !initial_dao_balance.is_zero() {
                    msgs.push(WasmMsg::Execute {
                        contract_addr: issuer_addr.clone(),
                        msg: to_binary(&IssuerExecuteMsg::Mint {
                            to_address: dao.to_string(),
                            amount: initial_dao_balance,
                        })?,
                        funds: vec![],
                    });
                }
            }

            // Begin update issuer contract owner to be the DAO, this is a
            // two-step ownership transfer.
            // DAO must pass a prop to Accept Ownership
            msgs.push(WasmMsg::Execute {
                contract_addr: issuer_addr.clone(),
                msg: to_binary(&IssuerExecuteMsg::UpdateOwnership(
                    cw_ownable::Action::TransferOwnership {
                        new_owner: dao.to_string(),
                        expiry: None,
                    },
                ))?,
                funds: vec![],
            });

            Ok(Response::new()
                .add_messages(msgs)
                .set_data(to_binary(&TokenFactoryCallback {
                    denom,
                    token_contract: Some(issuer_addr.to_string()),
                })?))
        }
        INSTANTIATE_NFT_REPLY_ID => {
            // Parse nft address from instantiate reply
            let nft_address = parse_reply_instantiate_data(msg)?.contract_address;

            Ok(Response::new().set_data(to_binary(&NftFactoryCallback {
                nft_contract: nft_address.to_string(),
            })?))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
