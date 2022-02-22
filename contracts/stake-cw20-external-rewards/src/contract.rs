use crate::msg::{
    ExecuteMsg, InfoResponse, InstantiateMsg, PendingRewardsResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{
    Config, RewardConfig, CONFIG, LAST_UPDATE_BLOCK, PENDING_REWARDS, REWARD_CONFIG,
    REWARD_PER_TOKEN, USER_REWARD_PER_TOKEN,
};
use crate::ContractError;
use crate::ContractError::{NoRewardsClaimable, Unauthorized};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, Denom};
use stake_cw20::hooks::StakeChangedHookMsg;

use std::cmp::{max, min};

const CONTRACT_NAME: &str = "crates.io:stake_cw20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = match msg.admin {
        Some(admin) => Some(deps.api.addr_validate(admin.as_str())?),
        None => None,
    };

    let config = Config {
        admin,
        staking_contract: msg.staking_contract,
        reward_token: msg.reward_token,
    };
    CONFIG.save(deps.storage, &config)?;

    let reward_config = RewardConfig {
        periodFinish: 0,
        rewardRate: Default::default(),
        rewardDuration: 100000,
    };
    REWARD_CONFIG.save(deps.storage, &reward_config);

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
        ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info, msg),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::Fund {} => execute_fund_native(deps, env, info),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response<Empty>, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(&*wrapper.sender)?;
    if config.reward_token != Denom::Cw20(info.sender) {
        return Err(Unauthorized {});
    };
    if config.admin != Some(sender.clone()) {
        return Err(Unauthorized {});
    };
    match msg {
        ReceiveMsg::Fund { .. } => execute_fund(deps, env, sender, wrapper.amount),
    }
}

pub fn execute_fund_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.admin != Some(info.sender.clone()) {
        return Err(Unauthorized {});
    };
    // TODO: Better error handling here
    let coin = info.funds.first().unwrap();
    let amount = coin.clone().amount;
    let denom = coin.clone().denom;
    if config.reward_token != Denom::Native(denom) {
        return Err(Unauthorized {});
    };
    execute_fund(deps, env, info.sender, amount)
}

pub fn execute_fund(
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response<Empty>, ContractError> {
    update_rewards(&mut deps, &env, &sender);

    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let new_reward_config = if reward_config.periodFinish < env.block.height {
        RewardConfig {
            periodFinish: env.block.height + reward_config.rewardDuration,
            rewardRate: amount / Uint128::from(reward_config.rewardDuration),
            rewardDuration: reward_config.rewardDuration,
        }
    } else {
        RewardConfig {
            periodFinish: reward_config.periodFinish,
            rewardRate: reward_config.rewardRate
                + (amount / Uint128::from(reward_config.periodFinish - env.block.height)),
            rewardDuration: reward_config.rewardDuration,
        }
    };

    REWARD_CONFIG.save(deps.storage, &new_reward_config);

    Ok(Response::new()
        .add_attribute("action", "fund")
        .add_attribute("amount", amount))
}

pub fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.staking_contract {
        return Err(ContractError::Unauthorized {});
    };
    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr),
        StakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr),
    }
}

pub fn execute_stake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
) -> Result<Response<Empty>, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "stake"))
}

pub fn execute_unstake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
) -> Result<Response<Empty>, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "unstake"))
}

pub fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
    update_rewards(&mut deps, &env, &info.sender);
    let rewards = PENDING_REWARDS
        .load(deps.storage, info.sender.clone())
        .map_err(|_| NoRewardsClaimable {})?;
    PENDING_REWARDS.save(deps.storage, info.sender.clone(), &Uint128::zero());
    let config = CONFIG.load(deps.storage)?;
    let transfer_msg = get_transfer_msg(info.sender, rewards, config.reward_token)?;
    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", rewards))
}

pub fn get_transfer_msg(recipient: Addr, amount: Uint128, denom: Denom) -> StdResult<CosmosMsg> {
    match denom {
        Denom::Native(denom) => Ok(BankMsg::Send {
            to_address: recipient.into_string(),
            amount: vec![Coin {
                denom: denom,
                amount,
            }],
        }
        .into()),
        Denom::Cw20(addr) => {
            let cw20_msg = to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: recipient.into_string(),
                amount,
            })?;
            Ok(WasmMsg::Execute {
                contract_addr: addr.into_string(),
                msg: cw20_msg,
                funds: vec![],
            }
            .into())
        }
    }
}

pub fn update_rewards(deps: &mut DepsMut, env: &Env, addr: &Addr) -> StdResult<()> {
    let config = CONFIG.load(deps.storage)?;
    let reward_per_token = get_reward_per_token(deps.as_ref(), env, &config.staking_contract)?;
    REWARD_PER_TOKEN.save(deps.storage, &reward_per_token);

    let earned_rewards = get_rewards_earned(
        deps.as_ref(),
        env,
        addr,
        reward_per_token,
        &config.staking_contract,
    )?;
    PENDING_REWARDS.update::<_, StdError>(deps.storage, addr.clone(), |r| {
        Ok(r.unwrap_or_default() + earned_rewards)
    });

    USER_REWARD_PER_TOKEN.save(deps.storage, addr.clone(), &reward_per_token);
    LAST_UPDATE_BLOCK.save(deps.storage, &env.block.height)?;
    Ok({})
}

pub fn get_reward_per_token(deps: Deps, env: &Env, staking_contract: &Addr) -> StdResult<Uint128> {
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let total_staked = get_total_staked(deps, staking_contract)?;
    let current_block = min(env.block.height, reward_config.periodFinish);
    let last_update_block = LAST_UPDATE_BLOCK.load(deps.storage).unwrap_or_default();
    let prev_reward_per_token = REWARD_PER_TOKEN.load(deps.storage).unwrap_or_default();
    let additional_reward_per_token = if total_staked == Uint128::zero() {
        Uint128::zero()
    } else {
        (reward_config.rewardRate
            * max(
                Uint128::from(current_block - last_update_block),
                Uint128::zero(),
            ))
            / total_staked
    };

    Ok(prev_reward_per_token + additional_reward_per_token)
}

pub fn get_rewards_earned(
    deps: Deps,
    _env: &Env,
    addr: &Addr,
    reward_per_token: Uint128,
    staking_contract: &Addr,
) -> StdResult<Uint128> {
    let _config = CONFIG.load(deps.storage)?;
    let staked_balance = get_staked_balance(deps, staking_contract, addr)?;
    let user_reward_per_token = USER_REWARD_PER_TOKEN
        .load(deps.storage, addr.clone())
        .unwrap_or_default();

    Ok((reward_per_token - user_reward_per_token) * staked_balance)
}

fn get_total_staked(deps: Deps, contract_addr: &Addr) -> StdResult<Uint128> {
    let msg = stake_cw20::msg::QueryMsg::TotalStakedAtHeight { height: None };
    let resp: stake_cw20::msg::TotalStakedAtHeightResponse =
        deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.total)
}

fn get_staked_balance(deps: Deps, contract_addr: &Addr, addr: &Addr) -> StdResult<Uint128> {
    let msg = stake_cw20::msg::QueryMsg::StakedBalanceAtHeight {
        address: addr.into(),
        height: None,
    };
    let resp: stake_cw20::msg::StakedBalanceAtHeightResponse =
        deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.balance)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_binary(&query_info(deps, env)?)?),
        QueryMsg::GetPendingRewards { address } => {
            Ok(to_binary(&query_pending_rewards(deps, env, address)?)?)
        }
    }
}

pub fn query_info(deps: Deps, _env: Env) -> StdResult<InfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let reward = REWARD_CONFIG.load(deps.storage)?;
    Ok(InfoResponse { config, reward })
}

pub fn query_pending_rewards(
    deps: Deps,
    env: Env,
    addr: Addr,
) -> StdResult<PendingRewardsResponse> {
    let config = CONFIG.load(deps.storage)?;
    let reward_per_token = get_reward_per_token(deps, &env, &config.staking_contract)?;
    let earned_rewards = get_rewards_earned(
        deps,
        &env,
        &addr,
        reward_per_token,
        &config.staking_contract,
    )?;
    let existing_rewards = PENDING_REWARDS.load(deps.storage, addr.clone())?;
    let pending_rewards = earned_rewards + existing_rewards;
    Ok(PendingRewardsResponse {
        address: addr,
        pending_rewards,
        denom: config.reward_token,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_update_config() {}
}
