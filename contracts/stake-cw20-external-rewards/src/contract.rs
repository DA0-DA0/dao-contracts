use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{
    Config, RewardConfig, CONFIG, LAST_UPDATE_BLOCK, PENDING_REWARDS, REWARD_CONFIG,
    REWARD_PER_TOKEN, USER_REWARD_PER_TOKEN,
};
use crate::ContractError;
use crate::ContractError::{NoRewardsClaimable, Unauthorized};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128, WasmMsg, BankMsg, Coin, from_binary};
use cw2::set_contract_version;
use stake_cw20::hooks::StakeChangedHookMsg;
use std::borrow::Borrow;
use std::cmp::{max, min};
use std::io::Empty;
use cosmwasm_std::OverflowOperation::Add;
use cw20::{Cw20ReceiveMsg, Denom};

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
    
    let reward_config = RewardConfig{
        periodFinish: 0,
        rewardRate: Default::default(),
        rewardDuration: 100000
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
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg)
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg
) -> Result<Response<Empty>, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(&*wrapper.sender)?;
    if config.reward_token != Denom::Cw20(info.sender) {
        return Err(Unauthorized {})
    };
    if config.admin != Some(sender.clone()){
        return Err(Unauthorized {})
    };
    match msg {
        ReceiveMsg::Fund { .. } => {execute_fund(deps, env, sender, wrapper.amount)}
    }
}

pub fn execute_fund_native (
    deps: DepsMut,
    env: Env,
    info: MessageInfo
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.admin != Some(info.sender.clone()){
        return Err(Unauthorized {})
    };
    let coin = info.funds.first()?;
    let amount = coin.clone().amount;
    let denom = coin.clone().denom;
    if config.reward_token != Denom::Native(denom) {
        return Err(Unauthorized {})
    };
    execute_fund(deps, env, info.sender, amount)
}

pub fn execute_fund (
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128
) -> Result<Response<Empty>, ContractError> {
    update_rewards(&mut deps, &env, &sender);
    
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let new_reward_config = if reward_config.periodFinish < env.block.height {
       RewardConfig {
           periodFinish: env.block.height + reward_config.rewardDuration,
           rewardRate: amount / reward_config.rewardDuration,
           rewardDuration: reward_config.rewardDuration
       }
    } else {
        RewardConfig {
            periodFinish: reward_config.periodFinish,
            rewardRate: reward_config.rewardRate + (amount / (reward_config.periodFinish - env.block.height)),
            rewardDuration: reward_config.rewardDuration
        }
    };

    REWARD_CONFIG.save(deps.storage, &new_reward_config);

    Ok(Response::new().add_attribute("action", "fund").add_attribute("amount", amount).add_attribute("reward_config", reward_config))
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
    addr: Addr
) -> Result<Response<Empty>, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "stake"))
}

pub fn execute_unstake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr
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
    Ok(Response::new().add_message(transfer_msg).add_attribute("action", "claim").add_attribute("amount", rewards))
}

pub fn get_transfer_msg(recipient: Addr, amount: Uint128, denom: Denom) -> StdResult<CosmosMsg> {
    match denom {
        Denom::Native(denom) => {
            Ok(
               BankMsg::Send {
                   to_address: recipient.into_string(),
                   amount: vec![Coin{ denom: denom, amount }]
               }.into()
            )
        }
        Denom::Cw20(addr) => {
            let cw20_msg = to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: recipient.into_string(),
                amount,
            })?;
            Ok(WasmMsg::Execute {
                contract_addr: addr.into_string(),
                msg: cw20_msg,
                funds: vec![],
            }.into())
        }
    }
}

pub fn update_rewards(deps: &mut DepsMut, env: &Env, addr: &Addr) -> StdResult<()> {
    let config = CONFIG.load(deps.storage)?;
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let total_staked = get_total_staked(deps.as_ref(), &config.staking_contract)?;
    let staked_balance = get_staked_balance(deps.as_ref(), &config.staking_contract, addr)?;
    let current_block = min(env.block.height, reward_config.periodFinish);
    let last_update_block = LAST_UPDATE_BLOCK.load(deps.storage).unwrap_or_default();

    let reward_per_token = REWARD_PER_TOKEN.update::<_, StdError>(deps.storage, |r| {
        let additional_reward_per_token = match total_staked {
            Uint128::zero() => Uint128::zero(),
            _ => ((reward_config.rewardRate * max(Uint128::from(current_block - last_update_block), Uint128::zero()))
                / total_staked)
        };
        Ok(
            r + additional_reward_per_token
        )
    })?;

    let user_reward_per_token = USER_REWARD_PER_TOKEN
        .load(deps.storage, addr.clone())
        .unwrap_or_default();

    PENDING_REWARDS.update::<_, StdError>(deps.storage, addr.clone(), |r| {
        Ok(r.unwrap_or_default() + ((reward_per_token - user_reward_per_token) * staked_balance))
    });

    USER_REWARD_PER_TOKEN.save(deps.storage, addr.clone(), &reward_per_token);
    LAST_UPDATE_BLOCK.save(deps.storage, &env.block.height)?;
    Ok({})
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
        QueryMsg::Test {} => Ok(to_binary(&1)?),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_update_config() {}
}
