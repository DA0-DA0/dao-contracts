#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    coins, to_binary, BankMsg, BankQuery, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdResult, SubMsg, Uint128, Uint256,
};
use cw2::set_contract_version;
use cw_controllers::ClaimsResponse;
use cw_utils::{must_pay, Duration};
use dao_interface::state::Admin;
use dao_interface::voting::{
    IsActiveResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use dao_voting::threshold::ActiveThreshold;
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery, TokenMsg, TokenQuerier};

use crate::error::ContractError;
use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
use crate::msg::{
    ActiveThresholdResponse, DenomResponse, ExecuteMsg, GetHooksResponse, InstantiateMsg,
    ListStakersResponse, MigrateMsg, NewTokenInfo, QueryMsg, StakerBalanceResponse, TokenInfo,
};
use crate::state::{
    Config, ACTIVE_THRESHOLD, CLAIMS, CONFIG, DAO, DENOM, HOOKS, MAX_CLAIMS, STAKED_BALANCES,
    STAKED_TOTAL, TOKEN_INSTANTIATION_INFO,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-native-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const CREATE_DENOM_REPLY_ID: u64 = 0;

// We multiply by this when calculating needed power for being active
// when using active threshold with percent
const PRECISION_FACTOR: u128 = 10u128.pow(9);

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
    deps: DepsMut<TokenFactoryQuery>,
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

    if let Some(active_threshold) = msg.active_threshold.as_ref() {
        if let ActiveThreshold::Percentage { percent } = active_threshold {
            if *percent > Decimal::percent(100) || *percent <= Decimal::percent(0) {
                return Err(ContractError::InvalidActivePercentage {});
            }
        }
        ACTIVE_THRESHOLD.save(deps.storage, active_threshold)?;
    }

    match msg.token_info {
        TokenInfo::Existing { denom } => {
            if let Some(ActiveThreshold::AbsoluteCount { count }) = msg.active_threshold {
                assert_valid_absolute_count_threshold(deps.as_ref(), &denom, count)?;
            }

            DENOM.save(deps.storage, &denom)?;

            Ok(Response::<TokenFactoryMsg>::new()
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
        TokenInfo::New(token) => {
            // TODO investigate how much validation we need to do
            if token.subdenom.eq("") {
                // TODO replace with token factory errors
                return Err(ContractError::NothingToClaim {});
            }

            // Create Token Factory denom SubMsg
            let create_denom_msg = SubMsg::reply_on_success(
                TokenMsg::CreateDenom {
                    subdenom: token.clone().subdenom,
                    metadata: token.clone().metadata,
                },
                CREATE_DENOM_REPLY_ID,
            );

            // Save new token info for use in reply
            TOKEN_INSTANTIATION_INFO.save(deps.storage, &token)?;

            Ok(Response::<TokenFactoryMsg>::new()
                .add_attribute("method", "create_denom")
                .add_submessage(create_denom_msg))
        }
    }
}

pub fn assert_valid_absolute_count_threshold(
    deps: Deps<TokenFactoryQuery>,
    token_denom: &str,
    count: Uint128,
) -> Result<(), ContractError> {
    if count.is_zero() {
        return Err(ContractError::ZeroActiveCount {});
    }
    let supply: cosmwasm_std::SupplyResponse =
        deps.querier
            .query(&cosmwasm_std::QueryRequest::Bank(BankQuery::Supply {
                denom: token_denom.to_string(),
            }))?;
    if count > supply.amount.amount {
        return Err(ContractError::InvalidAbsoluteCount {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg {
        ExecuteMsg::Stake {} => execute_stake(deps, env, info),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(deps, info, owner, manager, duration),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::UpdateActiveThreshold { new_threshold } => {
            execute_update_active_threshold(deps, env, info, new_threshold)
        }
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
    }
}

pub fn execute_stake(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
    let hook_msgs = stake_hook_msgs(deps.storage, info.sender.clone(), amount)?;

    Ok(Response::<TokenFactoryMsg>::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("amount", amount.to_string())
        .add_attribute("from", info.sender))
}

pub fn execute_unstake(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), amount)?;

    let config = CONFIG.load(deps.storage)?;
    let denom = DENOM.load(deps.storage)?;
    match config.unstaking_duration {
        None => {
            let msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: coins(amount.u128(), denom),
            });
            Ok(Response::<TokenFactoryMsg>::new()
                .add_message(msg)
                .add_submessages(hook_msgs)
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
            Ok(Response::<TokenFactoryMsg>::new()
                .add_submessages(hook_msgs)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", format!("{duration}")))
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    new_owner: Option<String>,
    new_manager: Option<String>,
    duration: Option<Duration>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
    Ok(Response::<TokenFactoryMsg>::new()
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
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let release = CLAIMS.claim_tokens(deps.storage, &info.sender, &env.block, None)?;
    if release.is_zero() {
        return Err(ContractError::NothingToClaim {});
    }

    let denom = DENOM.load(deps.storage)?;
    let msg = CosmosMsg::<TokenFactoryMsg>::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: coins(release.u128(), denom),
    });

    Ok(Response::<TokenFactoryMsg>::new()
        .add_message(msg)
        .add_attribute("action", "claim")
        .add_attribute("from", info.sender)
        .add_attribute("amount", release))
}

pub fn execute_update_active_threshold(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    new_active_threshold: Option<ActiveThreshold>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(active_threshold) = new_active_threshold {
        match active_threshold {
            ActiveThreshold::Percentage { percent } => {
                if percent > Decimal::percent(100) || percent.is_zero() {
                    return Err(ContractError::InvalidActivePercentage {});
                }
            }
            ActiveThreshold::AbsoluteCount { count } => {
                let denom = DENOM.load(deps.storage)?;
                assert_valid_absolute_count_threshold(deps.as_ref(), &denom, count)?;
            }
        }
        ACTIVE_THRESHOLD.save(deps.storage, &active_threshold)?;
    } else {
        ACTIVE_THRESHOLD.remove(deps.storage);
    }

    Ok(Response::<TokenFactoryMsg>::new().add_attribute("action", "update_active_threshold"))
}

pub fn execute_add_hook(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook)?;
    Ok(Response::<TokenFactoryMsg>::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook)?;
    Ok(Response::<TokenFactoryMsg>::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<TokenFactoryQuery>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
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
        QueryMsg::GetDenom {} => to_binary(&DenomResponse {
            denom: DENOM.load(deps.storage)?,
        }),
        QueryMsg::ListStakers { start_after, limit } => {
            query_list_stakers(deps, start_after, limit)
        }
        QueryMsg::IsActive {} => query_is_active(deps),
        QueryMsg::ActiveThreshold {} => query_active_threshold(deps),
        QueryMsg::GetHooks {} => to_binary(&query_hooks(deps)?),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps<TokenFactoryQuery>,
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
    deps: Deps<TokenFactoryQuery>,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = STAKED_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(TotalPowerAtHeightResponse { power, height })
}

pub fn query_info(deps: Deps<TokenFactoryQuery>) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps<TokenFactoryQuery>) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_binary(&dao)
}

pub fn query_claims(deps: Deps<TokenFactoryQuery>, address: String) -> StdResult<ClaimsResponse> {
    CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_list_stakers(
    deps: Deps<TokenFactoryQuery>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    unimplemented!()

    // let start_at = start_after
    //     .map(|addr| deps.api.addr_validate(&addr))
    //     .transpose()?;

    // // TODO fix me
    // let stakers = cw_paginate_storage::paginate_snapshot_map(
    //     deps,
    //     &STAKED_BALANCES,
    //     start_at.as_ref(),
    //     limit,
    //     cosmwasm_std::Order::Ascending,
    // )?;

    // let stakers = stakers
    //     .into_iter()
    //     .map(|(address, balance)| StakerBalanceResponse {
    //         address: address.into_string(),
    //         balance,
    //     })
    //     .collect();

    // to_binary(&ListStakersResponse { stakers })
}

pub fn query_is_active(deps: Deps<TokenFactoryQuery>) -> StdResult<Binary> {
    let threshold = ACTIVE_THRESHOLD.may_load(deps.storage)?;
    if let Some(threshold) = threshold {
        let denom = DENOM.load(deps.storage)?;
        let actual_power = STAKED_TOTAL.may_load(deps.storage)?.unwrap_or_default();
        match threshold {
            ActiveThreshold::AbsoluteCount { count } => to_binary(&IsActiveResponse {
                active: actual_power >= count,
            }),
            ActiveThreshold::Percentage { percent } => {
                // percent is bounded between [0, 100]. decimal
                // represents percents in u128 terms as p *
                // 10^15. this bounds percent between [0, 10^17].
                //
                // total_potential_power is bounded between [0, 2^128]
                // as it tracks the balances of a cw20 token which has
                // a max supply of 2^128.
                //
                // with our precision factor being 10^9:
                //
                // total_power <= 2^128 * 10^9 <= 2^256
                //
                // so we're good to put that in a u256.
                //
                // multiply_ratio promotes to a u512 under the hood,
                // so it won't overflow, multiplying by a percent less
                // than 100 is gonna make something the same size or
                // smaller, applied + 10^9 <= 2^128 * 10^9 + 10^9 <=
                // 2^256, so the top of the round won't overflow, and
                // rounding is rounding down, so the whole thing can
                // be safely unwrapped at the end of the day thank you
                // for coming to my ted talk.
                let total_potential_power: cosmwasm_std::SupplyResponse =
                    deps.querier
                        .query(&cosmwasm_std::QueryRequest::Bank(BankQuery::Supply {
                            denom,
                        }))?;
                let total_power = total_potential_power
                    .amount
                    .amount
                    .full_mul(PRECISION_FACTOR);
                // under the hood decimals are `atomics / 10^decimal_places`.
                // cosmwasm doesn't give us a Decimal * Uint256
                // implementation so we take the decimal apart and
                // multiply by the fraction.
                let applied = total_power.multiply_ratio(
                    percent.atomics(),
                    Uint256::from(10u64).pow(percent.decimal_places()),
                );
                let rounded = (applied + Uint256::from(PRECISION_FACTOR) - Uint256::from(1u128))
                    / Uint256::from(PRECISION_FACTOR);
                let count: Uint128 = rounded.try_into().unwrap();
                to_binary(&IsActiveResponse {
                    active: actual_power >= count,
                })
            }
        }
    } else {
        to_binary(&IsActiveResponse { active: true })
    }
}

pub fn query_active_threshold(deps: Deps<TokenFactoryQuery>) -> StdResult<Binary> {
    to_binary(&ActiveThresholdResponse {
        active_threshold: ACTIVE_THRESHOLD.may_load(deps.storage)?,
    })
}

pub fn query_hooks(deps: Deps<TokenFactoryQuery>) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::<TokenFactoryMsg>::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    msg: Reply,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg.id {
        CREATE_DENOM_REPLY_ID => {
            // Load info for new token and the DAO's address
            let token = TOKEN_INSTANTIATION_INFO.load(deps.storage)?;
            let dao = DAO.load(deps.storage)?;

            // Get the new token factory denom
            let querier = TokenQuerier::new(&deps.querier);
            let denom = querier
                .full_denom(env.contract.address.to_string(), token.subdenom)?
                .denom;

            let mut mint_msgs: Vec<TokenMsg> = vec![];

            // Check supply is greater than zero
            let initial_supply = token
                .initial_balances
                .iter()
                .fold(Uint128::zero(), |p, n| p + n.amount);

            // Cannot instantiate with no initial token owners because it would
            // immediately lock the DAO.
            if initial_supply.is_zero() {
                return Err(ContractError::InitialBalancesError {});
            }

            // Mint initial balances
            token.initial_balances.iter().for_each(|b| {
                mint_msgs.push(TokenMsg::MintTokens {
                    denom: denom.clone(),
                    amount: b.amount,
                    mint_to_address: b.mint_to_address.clone(),
                })
            });

            // Add initial DAO balance to initial_balances if nonzero.
            if let Some(initial_dao_balance) = token.initial_dao_balance {
                if !initial_dao_balance.is_zero() {
                    mint_msgs.push(TokenMsg::MintTokens {
                        denom: denom.clone(),
                        amount: initial_dao_balance,
                        mint_to_address: dao.to_string(),
                    })
                }
            }

            // Clear up unneeded storage.
            TOKEN_INSTANTIATION_INFO.remove(deps.storage);

            // Update token factory denom admin to be the DAO
            let update_token_admin_msg = TokenMsg::ChangeAdmin {
                denom: denom.clone(),
                new_admin_address: dao.to_string(),
            };

            // TODO what other info do we want here?
            Ok(Response::new()
                .add_messages(mint_msgs)
                .add_message(update_token_admin_msg))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
