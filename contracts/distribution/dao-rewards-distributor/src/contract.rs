#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Denom::Cw20;
use cw20::{Cw20ReceiveMsg, Denom};
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};
use dao_interface::voting::{
    Query as VotingQueryMsg, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use std::cmp::min;
use std::convert::TryInto;

use crate::msg::{
    ExecuteMsg, InfoResponse, InstantiateMsg, PendingRewardsResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{
    Config, RewardConfig, CONFIG, LAST_UPDATE_BLOCK, PENDING_REWARDS, REWARD_CONFIG,
    REWARD_PER_TOKEN, USER_REWARD_PER_TOKEN,
};
use crate::ContractError;
use crate::ContractError::{
    InvalidCw20, InvalidFunds, NoRewardsClaimable, RewardPeriodNotFinished,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Intialize the contract owner
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    let reward_denom = match msg.reward_denom {
        Denom::Native(denom) => Denom::Native(denom),
        Cw20(addr) => Cw20(deps.api.addr_validate(addr.as_ref())?),
    };

    // Verify contract provided is a voting module contract
    let _: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &msg.vp_contract,
        &VotingQueryMsg::TotalPowerAtHeight { height: None },
    )?;

    // Optional hook caller is allowed to call voting power change hooks.
    // If not provided, only the voting power contract is used.
    let hook_caller: Option<Addr> = match msg.hook_caller {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };

    // Save the contract configuration
    let config = Config {
        vp_contract: deps.api.addr_validate(&msg.vp_contract)?,
        hook_caller,
        reward_denom,
    };
    CONFIG.save(deps.storage, &config)?;

    // Reward duration must be greater than 0
    if msg.reward_duration == 0 {
        return Err(ContractError::ZeroRewardDuration {});
    }

    // Initialize the reward config
    let reward_config = RewardConfig {
        period_finish: 0,
        reward_rate: Uint128::zero(),
        reward_duration: msg.reward_duration,
    };
    REWARD_CONFIG.save(deps.storage, &reward_config)?;

    Ok(Response::new()
        .add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string()))
        .add_attribute("vp_contract", config.vp_contract)
        .add_attribute(
            "reward_denom",
            match config.reward_denom {
                Denom::Native(denom) => denom,
                Cw20(addr) => addr.into_string(),
            },
        )
        .add_attribute("reward_rate", reward_config.reward_rate)
        .add_attribute("period_finish", reward_config.period_finish.to_string())
        .add_attribute("reward_duration", reward_config.reward_duration.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info, msg),
        ExecuteMsg::NftStakeChangeHook(msg) => execute_nft_stake_changed(deps, env, info, msg),
        ExecuteMsg::MemberChangedHook(msg) => execute_membership_changed(deps, env, info, msg),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::Fund {} => execute_fund_native(deps, env, info),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::UpdateRewardDuration { new_duration } => {
            execute_update_reward_duration(deps, env, info, new_duration)
        }
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_json(&wrapper.msg)?;
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(&wrapper.sender)?;

    // This method is only to be used by cw20 tokens
    if config.reward_denom != Denom::Cw20(info.sender) {
        return Err(InvalidCw20 {});
    };

    match msg {
        ReceiveMsg::Fund {} => execute_fund(deps, env, sender, wrapper.amount),
    }
}

pub fn execute_fund_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match config.reward_denom {
        Denom::Native(denom) => {
            // Check that the correct denom has been sent
            let amount = cw_utils::must_pay(&info, &denom).map_err(|_| InvalidFunds {})?;
            execute_fund(deps, env, info.sender, amount)
        }
        Cw20(_) => Err(InvalidFunds {}),
    }
}

pub fn execute_fund(
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Ensure that the sender is the owner
    cw_ownable::assert_owner(deps.storage, &sender)?;

    update_rewards(&mut deps, &env, &sender)?;

    let reward_config = REWARD_CONFIG.load(deps.storage)?;

    // Ensure that the current reward period has ended
    if reward_config.period_finish > env.block.height {
        return Err(RewardPeriodNotFinished {});
    }

    let new_reward_config = RewardConfig {
        period_finish: env.block.height + reward_config.reward_duration,
        reward_rate: amount
            .checked_div(Uint128::from(reward_config.reward_duration))
            .map_err(StdError::divide_by_zero)?,
        // As we're not changing the value and changing the value
        // validates that the duration is non-zero we don't need to
        // check here.
        reward_duration: reward_config.reward_duration,
    };

    if new_reward_config.reward_rate == Uint128::zero() {
        return Err(ContractError::RewardRateLessThenOnePerBlock {});
    };

    REWARD_CONFIG.save(deps.storage, &new_reward_config)?;
    LAST_UPDATE_BLOCK.save(deps.storage, &env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "fund")
        .add_attribute("amount", amount)
        .add_attribute("new_reward_rate", new_reward_config.reward_rate.to_string()))
}

pub fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    check_hook_caller(deps.as_ref(), info)?;

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr),
        StakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr),
    }
}

pub fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    check_hook_caller(deps.as_ref(), info)?;

    // Get the addresses of members whose voting power has changed.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        update_rewards(&mut deps, &env, &addr)?;
    }

    Ok(Response::new().add_attribute("action", "membership_changed"))
}

pub fn execute_nft_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: NftStakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    check_hook_caller(deps.as_ref(), info)?;

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr),
        NftStakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr),
    }
}

pub fn execute_stake(mut deps: DepsMut, env: Env, addr: Addr) -> Result<Response, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "stake"))
}

pub fn execute_unstake(mut deps: DepsMut, env: Env, addr: Addr) -> Result<Response, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "unstake"))
}

pub fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Update the rewards information for the sender.
    update_rewards(&mut deps, &env, &info.sender)?;

    // Get the pending rewards for the sender.
    let rewards = PENDING_REWARDS
        .load(deps.storage, info.sender.clone())
        .map_err(|_| NoRewardsClaimable {})?;

    // If there are no rewards to claim, return an error.
    if rewards == Uint128::zero() {
        return Err(ContractError::NoRewardsClaimable {});
    }

    // Save the pending rewards for the sender, it will now be zero.
    PENDING_REWARDS.save(deps.storage, info.sender.clone(), &Uint128::zero())?;

    let config = CONFIG.load(deps.storage)?;

    // Transfer the rewards to the sender.
    let transfer_msg = get_transfer_msg(info.sender, rewards, config.reward_denom)?;
    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", rewards))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    // Update the current contract owner.
    // Note, this is a two step process, the new owner must accept this ownership transfer.
    // First the owner specifies the new owner, then the new owner must accept.
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

/// Ensures hooks that update voting power are only called by the voting power contract
/// or the designated hook_caller contract (if configured).
pub fn check_hook_caller(deps: Deps, info: MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only the voting power contract or the designated hook_caller contract (if configured)
    // can call this hook.
    match config.hook_caller {
        Some(hook_caller) => {
            if info.sender != hook_caller {
                return Err(ContractError::InvalidHookSender {});
            }
        }
        None => {
            if info.sender != config.vp_contract {
                return Err(ContractError::InvalidHookSender {});
            };
        }
    }
    Ok(())
}

/// Returns the appropriate CosmosMsg for transferring the reward token.
pub fn get_transfer_msg(recipient: Addr, amount: Uint128, denom: Denom) -> StdResult<CosmosMsg> {
    match denom {
        Denom::Native(denom) => Ok(BankMsg::Send {
            to_address: recipient.into_string(),
            amount: vec![Coin { denom, amount }],
        }
        .into()),
        Denom::Cw20(addr) => {
            let cw20_msg = to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
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

    // Reward per token represents the amount of rewards per unit of voting power.
    let reward_per_token = get_reward_per_token(deps.as_ref(), env, &config.vp_contract)?;
    REWARD_PER_TOKEN.save(deps.storage, &reward_per_token)?;

    // The amount of rewards earned up until this point.
    let earned_rewards = get_rewards_earned(
        deps.as_ref(),
        env,
        addr,
        reward_per_token,
        &config.vp_contract,
    )?;

    // Update the users pending rewards
    PENDING_REWARDS.update::<_, StdError>(deps.storage, addr.clone(), |r| {
        Ok(r.unwrap_or_default() + earned_rewards)
    })?;

    // Update the users latest reward per token value.
    USER_REWARD_PER_TOKEN.save(deps.storage, addr.clone(), &reward_per_token)?;

    // Update the last time rewards were updated.
    let last_time_reward_applicable = get_last_time_reward_applicable(deps.as_ref(), env)?;
    LAST_UPDATE_BLOCK.save(deps.storage, &last_time_reward_applicable)?;

    Ok(())
}

pub fn get_reward_per_token(deps: Deps, env: &Env, vp_contract: &Addr) -> StdResult<Uint256> {
    let reward_config = REWARD_CONFIG.load(deps.storage)?;

    // Get the total voting power at this block height.
    let total_power = get_total_voting_power(deps, env, vp_contract)?;

    // Get information on the last time rewards were updated.
    let last_time_reward_applicable = get_last_time_reward_applicable(deps, env)?;
    let last_update_block = LAST_UPDATE_BLOCK.load(deps.storage).unwrap_or_default();

    // Get the amount of rewards per unit of voting power.
    let prev_reward_per_token = REWARD_PER_TOKEN.load(deps.storage).unwrap_or_default();

    let additional_reward_per_token = if total_power == Uint128::zero() {
        Uint256::zero()
    } else {
        // It is impossible for this to overflow as total rewards can never exceed max value of
        // Uint128 as total tokens in existence cannot exceed Uint128
        let numerator = reward_config
            .reward_rate
            .full_mul(Uint128::from(
                last_time_reward_applicable - last_update_block,
            ))
            .checked_mul(scale_factor())?;
        let denominator = Uint256::from(total_power);
        numerator.checked_div(denominator)?
    };

    Ok(prev_reward_per_token + additional_reward_per_token)
}

pub fn get_rewards_earned(
    deps: Deps,
    env: &Env,
    addr: &Addr,
    reward_per_token: Uint256,
    vp_contract: &Addr,
) -> StdResult<Uint128> {
    // Get the users voting power at the current height.
    let voting_power = Uint256::from(get_voting_power(deps, env, vp_contract, addr)?);

    // Load the users latest reward per token value.
    let user_reward_per_token = USER_REWARD_PER_TOKEN
        .load(deps.storage, addr.clone())
        .unwrap_or_default();
    // Calculate the difference between the current reward per token value and the users latest
    let reward_factor = reward_per_token.checked_sub(user_reward_per_token)?;

    // Calculate the amount of rewards earned.
    // voting_power * reward_factor / scale_factor
    Ok(voting_power
        .checked_mul(reward_factor)?
        .checked_div(scale_factor())?
        .try_into()?)
}

fn get_last_time_reward_applicable(deps: Deps, env: &Env) -> StdResult<u64> {
    let reward_config = REWARD_CONFIG.load(deps.storage)?;

    // Take the minimum of the current block height and the period finish height.
    Ok(min(env.block.height, reward_config.period_finish))
}

fn get_total_voting_power(deps: Deps, env: &Env, contract_addr: &Addr) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::TotalPowerAtHeight {
        height: Some(env.block.height),
    };
    let resp: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

fn get_voting_power(
    deps: Deps,
    env: &Env,
    contract_addr: &Addr,
    addr: &Addr,
) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::VotingPowerAtHeight {
        address: addr.into(),
        height: Some(env.block.height),
    };
    let resp: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

pub fn execute_update_reward_duration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_duration: u64,
) -> Result<Response, ContractError> {
    // Ensure the sender is the owner.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_config = REWARD_CONFIG.load(deps.storage)?;
    if reward_config.period_finish > env.block.height {
        return Err(ContractError::RewardPeriodNotFinished {});
    };

    if new_duration == 0 {
        return Err(ContractError::ZeroRewardDuration {});
    }

    let old_duration = reward_config.reward_duration;
    reward_config.reward_duration = new_duration;
    REWARD_CONFIG.save(deps.storage, &reward_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_reward_duration")
        .add_attribute("new_duration", new_duration.to_string())
        .add_attribute("old_duration", old_duration.to_string()))
}

fn scale_factor() -> Uint256 {
    Uint256::from(10u8).pow(39)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_json_binary(&query_info(deps, env)?)?),
        QueryMsg::GetPendingRewards { address } => {
            Ok(to_json_binary(&query_pending_rewards(deps, env, address)?)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
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
    addr: String,
) -> StdResult<PendingRewardsResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let config = CONFIG.load(deps.storage)?;
    let reward_per_token = get_reward_per_token(deps, &env, &config.vp_contract)?;
    let earned_rewards =
        get_rewards_earned(deps, &env, &addr, reward_per_token, &config.vp_contract)?;
    let existing_rewards = PENDING_REWARDS
        .load(deps.storage, addr.clone())
        .unwrap_or_default();
    let pending_rewards = earned_rewards + existing_rewards;
    Ok(PendingRewardsResponse {
        address: addr.to_string(),
        pending_rewards,
        denom: config.reward_denom,
        last_update_block: LAST_UPDATE_BLOCK.load(deps.storage).unwrap_or_default(),
    })
}
