#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Reply, Response, StdResult, SubMsg, Uint128,
};
use cw2::set_contract_version;
use cw_controllers::ClaimsResponse;
use cw_utils::{must_pay, Duration, ParseReplyError};
use dao_interface::state::Admin;
use dao_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};
use token_bindings::{
    CreateDenomResponse, DenomUnit, Metadata, MetadataResponse, TokenFactoryMsg, TokenFactoryQuery,
    TokenMsg, TokenQuery,
};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, ListStakersResponse, MigrateMsg, QueryMsg, StakerBalanceResponse,
    TokenInfo,
};
use crate::state::{
    Config, CLAIMS, CONFIG, DAO, DENOM, MAX_CLAIMS, STAKED_BALANCES, STAKED_TOTAL, TOKEN_INFO,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-native-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const CREATE_TOKEN_REPLY_ID: u64 = 0;

fn validate_duration(duration: Option<Duration>) -> Result<(), ContractError> {
    if let Some(unstaking_duration) = duration {
        match unstaking_duration {
            Duration::Height(height) => {
                if height == 0 {
                    return Err(ContractError::InvalidUnstakingDuration {});
                }
            }
            Duration::Time(time) => {
                if time == 0 {
                    return Err(ContractError::InvalidUnstakingDuration {});
                }
            }
        }
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = msg
        .owner
        .as_ref()
        .map(|owner| match owner {
            Admin::Address { addr } => deps.api.addr_validate(addr),
            Admin::CoreModule {} => Ok(info.sender.clone()),
        })
        .transpose()?;
    let manager = msg
        .manager
        .map(|manager| deps.api.addr_validate(&manager))
        .transpose()?;

    validate_duration(msg.unstaking_duration)?;

    let config = Config {
        owner,
        manager,
        unstaking_duration: msg.unstaking_duration,
    };

    CONFIG.save(deps.storage, &config)?;
    DAO.save(deps.storage, &info.sender)?;

    match &msg.token_info {
        TokenInfo::Existing { denom } => {
            DENOM.save(deps.storage, denom)?;

            Ok(Response::new()
                .add_attribute("action", "instantiate")
                .add_attribute("token", "existing_token")
                .add_attribute("token_denom", denom)
                .add_attribute(
                    "owner",
                    config
                        .owner
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                )
                .add_attribute(
                    "manager",
                    config
                        .manager
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ))
        }
        TokenInfo::New {
            symbol,
            initial_balances,
            ..
        } => {
            let initial_supply = initial_balances
                .iter()
                .fold(Uint128::zero(), |p, n| p + n.amount);
            // Cannot instantiate with no initial token owners because it would
            // immediately lock the DAO.
            if initial_supply.is_zero() {
                return Err(ContractError::InitialBalancesError {});
            }

            // Validate initial balance addresses.
            for initial in initial_balances {
                deps.api.addr_validate(&initial.address)?;
            }

            // Store token info for usage later in replies.
            TOKEN_INFO.save(deps.storage, &msg.token_info)?;

            let msg: SubMsg<TokenFactoryMsg> = SubMsg::reply_on_success(
                TokenMsg::CreateDenom {
                    subdenom: symbol.to_lowercase(),
                    metadata: None,
                },
                CREATE_TOKEN_REPLY_ID,
            );

            Ok(Response::default()
                .add_attribute("action", "instantiate")
                .add_attribute("token", "new_token")
                .add_attribute("token_denom", symbol.to_lowercase())
                .add_attribute(
                    "owner",
                    config
                        .owner
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                )
                .add_attribute(
                    "manager",
                    config
                        .manager
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                )
                .add_submessage(msg))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Stake {} => execute_stake(deps, env, info),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(deps, info, owner, manager, duration),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let denom = DENOM.load(deps.storage)?;
    let amount = must_pay(&info, &denom)?;

    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |balance| -> StdResult<Uint128> { Ok(balance.unwrap_or_default().checked_add(amount)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> { Ok(total.unwrap_or_default().checked_add(amount)?) },
    )?;

    Ok(Response::new()
        .add_attribute("action", "stake")
        .add_attribute("amount", amount.to_string())
        .add_attribute("from", info.sender))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::ZeroUnstake {});
    }

    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |balance| -> Result<Uint128, ContractError> {
            balance
                .unwrap_or_default()
                .checked_sub(amount)
                .map_err(|_e| ContractError::InvalidUnstakeAmount {})
        },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> Result<Uint128, ContractError> {
            total
                .unwrap_or_default()
                .checked_sub(amount)
                .map_err(|_e| ContractError::InvalidUnstakeAmount {})
        },
    )?;

    let config = CONFIG.load(deps.storage)?;
    let denom = DENOM.load(deps.storage)?;
    match config.unstaking_duration {
        None => {
            let msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: coins(amount.u128(), denom),
            });
            Ok(Response::new()
                .add_message(msg)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", "None"))
        }
        Some(duration) => {
            let outstanding_claims = CLAIMS.query_claims(deps.as_ref(), &info.sender)?.claims;
            if outstanding_claims.len() >= MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            CLAIMS.create_claim(
                deps.storage,
                &info.sender,
                amount,
                duration.after(&env.block),
            )?;
            Ok(Response::new()
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", format!("{duration}")))
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
    new_manager: Option<String>,
    duration: Option<Duration>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&new_owner))
        .transpose()?;
    let new_manager = new_manager
        .map(|new_manager| deps.api.addr_validate(&new_manager))
        .transpose()?;

    validate_duration(duration)?;

    if Some(info.sender) != config.owner && new_owner != config.owner {
        return Err(ContractError::OnlyOwnerCanChangeOwner {});
    };

    config.owner = new_owner;
    config.manager = new_manager;

    config.unstaking_duration = duration;

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute(
            "owner",
            config
                .owner
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute(
            "manager",
            config
                .manager
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let release = CLAIMS.claim_tokens(deps.storage, &info.sender, &env.block, None)?;
    if release.is_zero() {
        return Err(ContractError::NothingToClaim {});
    }

    let denom = DENOM.load(deps.storage)?;
    let msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: coins(release.u128(), denom),
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "claim")
        .add_attribute("from", info.sender)
        .add_attribute("amount", release))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::Claims { address } => to_binary(&query_claims(deps, address)?),
        QueryMsg::GetConfig {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::GetDenom {} => to_binary(&DENOM.load(deps.storage)?),
        QueryMsg::ListStakers { start_after, limit } => {
            query_list_stakers(deps, start_after, limit)
        }
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let address = deps.api.addr_validate(&address)?;
    let power = STAKED_BALANCES
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = STAKED_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(TotalPowerAtHeightResponse { power, height })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_binary(&dao)
}

pub fn query_claims(deps: Deps, address: String) -> StdResult<ClaimsResponse> {
    CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_list_stakers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let stakers = cw_paginate_storage::paginate_snapshot_map(
        deps,
        &STAKED_BALANCES,
        start_at.as_ref(),
        limit,
        cosmwasm_std::Order::Ascending,
    )?;

    let stakers = stakers
        .into_iter()
        .map(|(address, balance)| StakerBalanceResponse {
            address: address.into_string(),
            balance,
        })
        .collect();

    to_binary(&ListStakersResponse { stakers })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    msg: Reply,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg.id {
        CREATE_TOKEN_REPLY_ID => {
            let data = msg
                .result
                .into_result()
                .map_err(ParseReplyError::SubMsgFailure)?
                .data
                .ok_or_else(|| ParseReplyError::ParseFailure("Missing reply data".to_owned()))?;
            let res = CreateDenomResponse::from_reply_data(data);

            match res {
                Ok(res) => {
                    let token_info = TOKEN_INFO.load(deps.storage)?;
                    match token_info {
                        TokenInfo::New {
                            name,
                            symbol,
                            decimals,
                            initial_balances,
                            initial_dao_balance,
                        } => {
                            let dao = DAO.load(deps.storage)?;
                            let denom = res.new_token_denom;

                            let res: MetadataResponse =
                                deps.querier
                                    .query(&QueryRequest::from(TokenQuery::Metadata {
                                        denom: denom.clone(),
                                    }))?;
                            if res.metadata.is_none() {
                                return Err(ContractError::TokenCreationError {});
                            }
                            let mut metadata = res.metadata.unwrap();
                            metadata.denom_units.append(&mut vec![DenomUnit {
                                denom: symbol.clone(),
                                exponent: decimals,
                                aliases: vec![],
                            }]);

                            // Mint initial tokens.
                            let mut mint_msgs: Vec<TokenMsg> = initial_balances
                                .iter()
                                .map(|initial| TokenMsg::MintTokens {
                                    denom: denom.clone(),
                                    amount: initial.amount,
                                    mint_to_address: initial.address.clone(),
                                })
                                .collect();
                            // Mint initial DAO tokens.
                            if let Some(initial_dao_balance) = initial_dao_balance {
                                if initial_dao_balance > Uint128::zero() {
                                    mint_msgs.push(TokenMsg::MintTokens {
                                        denom: denom.clone(),
                                        amount: initial_dao_balance,
                                        mint_to_address: dao.to_string(),
                                    });
                                }
                            }
                            // Update the metadata with the symbol and decimals.
                            let metadata_msg = TokenMsg::SetMetadata {
                                denom: denom.clone(),
                                metadata: Metadata {
                                    description: metadata.description,
                                    denom_units: metadata.denom_units,
                                    base: metadata.base,
                                    display: Some(symbol.clone()),
                                    name: Some(name),
                                    symbol: Some(symbol),
                                },
                            };
                            // Set the token's admin to the DAO.
                            let admin_msg = TokenMsg::ChangeAdmin {
                                denom: denom.clone(),
                                new_admin_address: dao.to_string(),
                            };

                            Ok(Response::default()
                                .add_attribute("token_denom", denom)
                                .add_messages(mint_msgs)
                                .add_message(metadata_msg)
                                .add_message(admin_msg))
                        }
                        _ => Err(ContractError::TokenCreationError {}),
                    }
                }
                Err(_) => Err(ContractError::TokenCreationError {}),
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
