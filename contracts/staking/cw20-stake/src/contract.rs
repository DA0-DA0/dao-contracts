use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_binary, from_slice, to_binary, to_vec, Addr, Binary, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdError, StdResult, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
use crate::math;
use crate::msg::{
    ExecuteMsg, GetHooksResponse, InstantiateMsg, ListStakersResponse, MigrateMsg, QueryMsg,
    ReceiveMsg, StakedBalanceAtHeightResponse, StakedValueResponse, StakerBalanceResponse,
    TotalStakedAtHeightResponse, TotalValueResponse,
};
use crate::state::{
    Config, BALANCE, CLAIMS, CONFIG, HOOKS, MAX_CLAIMS, STAKED_BALANCES, STAKED_TOTAL,
};
use crate::ContractError;
use cw2::set_contract_version;
pub use cw20_base::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
pub use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, execute_update_marketing,
    execute_upload_logo, query_balance, query_download_logo, query_marketing_info, query_minter,
    query_token_info,
};
pub use cw20_base::enumerable::{query_all_accounts, query_owner_allowances};
use cw_controllers::ClaimsResponse;
use cw_utils::Duration;

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw20-stake";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let owner = match msg.owner {
        Some(owner) => Some(deps.api.addr_validate(owner.as_str())?),
        None => None,
    };

    let manager = match msg.manager {
        Some(manager) => Some(deps.api.addr_validate(manager.as_str())?),
        None => None,
    };

    validate_duration(msg.unstaking_duration)?;
    let config = Config {
        owner,
        manager,
        token_address: deps.api.addr_validate(&msg.token_address)?,
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;

    // Initialize state to zero. We do this instead of using
    // `unwrap_or_default` where this is used as it protects us
    // against a scenerio where state is cleared by a bad actor and
    // `unwrap_or_default` carries on.
    STAKED_TOTAL.save(deps.storage, &Uint128::zero(), env.block.height)?;
    BALANCE.save(deps.storage, &Uint128::zero())?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(info, deps, owner, manager, duration),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
    }
}

pub fn execute_update_config(
    info: MessageInfo,
    deps: DepsMut,
    new_owner: Option<String>,
    new_manager: Option<String>,
    duration: Option<Duration>,
) -> Result<Response, ContractError> {
    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&new_owner))
        .transpose()?;
    let new_manager = new_manager
        .map(|new_manager| deps.api.addr_validate(&new_manager))
        .transpose()?;
    let mut config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    };
    if Some(info.sender) != config.owner && new_owner != config.owner {
        return Err(ContractError::OnlyOwnerCanChangeOwner {});
    };

    validate_duration(duration)?;

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

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.token_address {
        return Err(ContractError::InvalidToken {
            received: info.sender,
            expected: config.token_address,
        });
    }
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    match msg {
        ReceiveMsg::Stake {} => execute_stake(deps, env, sender, wrapper.amount),
        ReceiveMsg::Fund {} => execute_fund(deps, env, &sender, wrapper.amount),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let balance = BALANCE.load(deps.storage)?;
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    let amount_to_stake = math::amount_to_stake(staked_total, balance, amount);
    STAKED_BALANCES.update(
        deps.storage,
        &sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_add(amount_to_stake)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> {
            // Initialized during instantiate - OK to unwrap.
            Ok(total.unwrap().checked_add(amount_to_stake)?)
        },
    )?;
    BALANCE.save(
        deps.storage,
        &balance.checked_add(amount).map_err(StdError::overflow)?,
    )?;
    let hook_msgs = stake_hook_msgs(deps.storage, sender.clone(), amount_to_stake)?;
    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
        .add_attribute("amount", amount))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let balance = BALANCE.load(deps.storage)?;
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    // invariant checks for amount_to_claim
    if staked_total.is_zero() {
        return Err(ContractError::NothingStaked {});
    }
    if amount.saturating_add(balance) == Uint128::MAX {
        return Err(ContractError::Cw20InvaraintViolation {});
    }
    if amount > staked_total {
        return Err(ContractError::ImpossibleUnstake {});
    }
    let amount_to_claim = math::amount_to_claim(staked_total, balance, amount);
    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_sub(amount)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> {
            // Initialized during instantiate - OK to unwrap.
            Ok(total.unwrap().checked_sub(amount)?)
        },
    )?;
    BALANCE.save(
        deps.storage,
        &balance
            .checked_sub(amount_to_claim)
            .map_err(StdError::overflow)?,
    )?;
    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), amount)?;
    match config.unstaking_duration {
        None => {
            let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: amount_to_claim,
            };
            let wasm_msg = cosmwasm_std::WasmMsg::Execute {
                contract_addr: config.token_address.to_string(),
                msg: to_binary(&cw_send_msg)?,
                funds: vec![],
            };
            Ok(Response::new()
                .add_message(wasm_msg)
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
                amount_to_claim,
                duration.after(&env.block),
            )?;
            Ok(Response::new()
                .add_attribute("action", "unstake")
                .add_submessages(hook_msgs)
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", format!("{duration}")))
        }
    }
}

pub fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let release = CLAIMS.claim_tokens(deps.storage, &info.sender, &_env.block, None)?;
    if release.is_zero() {
        return Err(ContractError::NothingToClaim {});
    }
    let config = CONFIG.load(deps.storage)?;
    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: release,
    };
    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_binary(&cw_send_msg)?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "claim")
        .add_attribute("from", info.sender)
        .add_attribute("amount", release))
}

pub fn execute_fund(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    BALANCE.update(deps.storage, |balance| -> StdResult<_> {
        balance.checked_add(amount).map_err(StdError::overflow)
    })?;
    Ok(Response::new()
        .add_attribute("action", "fund")
        .add_attribute("from", sender)
        .add_attribute("amount", amount))
}

pub fn execute_add_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };
    HOOKS.add_hook(deps.storage, addr.clone())?;
    Ok(Response::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };
    HOOKS.remove_hook(deps.storage, addr.clone())?;
    Ok(Response::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::StakedBalanceAtHeight { address, height } => {
            to_binary(&query_staked_balance_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalStakedAtHeight { height } => {
            to_binary(&query_total_staked_at_height(deps, env, height)?)
        }
        QueryMsg::StakedValue { address } => to_binary(&query_staked_value(deps, env, address)?),
        QueryMsg::TotalValue {} => to_binary(&query_total_value(deps, env)?),
        QueryMsg::Claims { address } => to_binary(&query_claims(deps, address)?),
        QueryMsg::GetHooks {} => to_binary(&query_hooks(deps)?),
        QueryMsg::ListStakers { start_after, limit } => {
            query_list_stakers(deps, start_after, limit)
        }
    }
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<StakedBalanceAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let balance = STAKED_BALANCES
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(StakedBalanceAtHeightResponse { balance, height })
}

pub fn query_total_staked_at_height(
    deps: Deps,
    _env: Env,
    height: Option<u64>,
) -> StdResult<TotalStakedAtHeightResponse> {
    let height = height.unwrap_or(_env.block.height);
    let total = STAKED_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(TotalStakedAtHeightResponse { total, height })
}

pub fn query_staked_value(
    deps: Deps,
    _env: Env,
    address: String,
) -> StdResult<StakedValueResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCE.load(deps.storage).unwrap_or_default();
    let staked = STAKED_BALANCES
        .load(deps.storage, &address)
        .unwrap_or_default();
    let total = STAKED_TOTAL.load(deps.storage)?;
    if balance == Uint128::zero() || staked == Uint128::zero() || total == Uint128::zero() {
        Ok(StakedValueResponse {
            value: Uint128::zero(),
        })
    } else {
        let value = staked
            .checked_mul(balance)
            .map_err(StdError::overflow)?
            .checked_div(total)
            .map_err(StdError::divide_by_zero)?;
        Ok(StakedValueResponse { value })
    }
}

pub fn query_total_value(deps: Deps, _env: Env) -> StdResult<TotalValueResponse> {
    let balance = BALANCE.load(deps.storage)?;
    Ok(TotalValueResponse { total: balance })
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_claims(deps: Deps, address: String) -> StdResult<ClaimsResponse> {
    CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

pub fn query_list_stakers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let stakers = cw_paginate::paginate_snapshot_map(
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
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    #[cw_serde]
    struct BetaConfig {
        pub admin: Addr,
        pub token_address: Addr,
        pub unstaking_duration: Option<Duration>,
    }

    match msg {
        MigrateMsg::FromBeta { manager } => {
            let data = deps
                .storage
                .get(b"config")
                .ok_or_else(|| StdError::not_found("config"))?;
            let beta_config: BetaConfig = from_slice(&data)?;
            let new_config = Config {
                owner: Some(beta_config.admin),
                manager: manager
                    .map(|human| deps.api.addr_validate(&human))
                    .transpose()?,
                token_address: beta_config.token_address,
                unstaking_duration: beta_config.unstaking_duration,
            };
            deps.storage.set(b"config", &to_vec(&new_config)?);
            Ok(Response::default())
        }
        MigrateMsg::FromCompatible {} => Ok(Response::default()),
    }
}
