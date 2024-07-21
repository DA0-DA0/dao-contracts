#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128, Uint256,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw_utils::{one_coin, Duration, Expiration};
use dao_interface::voting::InfoResponse;

use std::collections::HashMap;

use crate::helpers::{get_duration_scalar, get_transfer_msg, validate_voting_power_contract};
use crate::hooks::{
    execute_membership_changed, execute_nft_stake_changed, execute_stake_changed,
    subscribe_denom_to_hook, unsubscribe_denom_from_hook,
};
use crate::msg::{
    ExecuteMsg, InstantiateMsg, PendingRewardsResponse, QueryMsg, ReceiveMsg, RegisterDenomMsg,
    RewardEmissionRate, RewardsStateResponse,
};
use crate::rewards::{
    get_accrued_rewards_since_last_user_action, get_active_total_earned_puvp, update_rewards,
};
use crate::state::{DenomRewardState, Epoch, DENOM_REWARD_STATES, USER_REWARD_STATES};
use crate::ContractError;

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

    Ok(Response::new().add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string())))
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
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::RegisterDenom(register_msg) => execute_register_denom(deps, info, register_msg),
        ExecuteMsg::UpdateDenom {
            denom,
            emission_rate,
            vp_contract,
            hook_caller,
            withdraw_destination,
        } => execute_update_denom(
            deps,
            env,
            info,
            denom,
            emission_rate,
            vp_contract,
            hook_caller,
            withdraw_destination,
        ),
        ExecuteMsg::Fund {} => execute_fund_native(deps, env, info),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Claim { denom } => execute_claim(deps, env, info, denom),
        ExecuteMsg::Withdraw { denom } => execute_withdraw(deps, info, env, denom),
    }
}

/// registers a new denom for rewards distribution.
/// only the owner can register a new denom.
/// a denom can only be registered once; update if you need to change something.
fn execute_register_denom(
    deps: DepsMut,
    info: MessageInfo,
    msg: RegisterDenomMsg,
) -> Result<Response, ContractError> {
    // only the owner can register a new denom
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Reward duration must be greater than 0 seconds/blocks
    if get_duration_scalar(&msg.emission_rate.duration) == 0 {
        return Err(ContractError::ZeroRewardDuration {});
    }

    let checked_denom = msg.denom.into_checked(deps.as_ref())?;
    let hook_caller = deps.api.addr_validate(&msg.hook_caller)?;
    let vp_contract = validate_voting_power_contract(&deps, msg.vp_contract)?;

    let withdraw_destination = match msg.withdraw_destination {
        // if withdraw destination is specified, we validate it
        Some(addr) => deps.api.addr_validate(&addr)?,
        // otherwise default to the owner
        None => info.sender,
    };

    // Initialize the reward state
    let reward_state = DenomRewardState {
        denom: checked_denom,
        active_epoch: Epoch {
            started_at: Expiration::Never {},
            ends_at: Expiration::Never {},
            emission_rate: msg.emission_rate,
            total_earned_puvp: Uint256::zero(),
            finish_block: None,
        },
        last_update: Expiration::Never {},
        vp_contract,
        hook_caller: hook_caller.clone(),
        funded_amount: Uint128::zero(),
        withdraw_destination,
        historical_earned_puvp: Uint256::zero(),
    };
    let str_denom = reward_state.to_str_denom();

    // store the new reward denom state or error if it already exists
    DENOM_REWARD_STATES.update(
        deps.storage,
        str_denom.to_string(),
        |existing| match existing {
            Some(_) => Err(ContractError::DenomAlreadyRegistered {}),
            None => Ok(reward_state),
        },
    )?;

    // update the registered hooks to include the new denom
    subscribe_denom_to_hook(deps.storage, str_denom, hook_caller.clone())?;

    Ok(Response::default().add_attribute("action", "register_reward_denom"))
}

/// updates the config for a registered denom
#[allow(clippy::too_many_arguments)]
fn execute_update_denom(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    emission_rate: Option<RewardEmissionRate>,
    vp_contract: Option<String>,
    hook_caller: Option<String>,
    withdraw_destination: Option<String>,
) -> Result<Response, ContractError> {
    // only the owner can update a denom config
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_state = DENOM_REWARD_STATES
        .load(deps.storage, denom.clone())
        .map_err(|_| ContractError::DenomNotRegistered {})?;

    if let Some(emission_rate) = emission_rate {
        // transition the epoch to the new emission rate
        reward_state.transition_epoch(deps.as_ref(), emission_rate, &env.block)?;
    }

    if let Some(vp_contract) = vp_contract {
        reward_state.vp_contract = validate_voting_power_contract(&deps, vp_contract)?;
    }

    if let Some(hook_caller) = hook_caller {
        // remove existing from registered hooks
        unsubscribe_denom_from_hook(deps.storage, &denom, reward_state.hook_caller)?;

        reward_state.hook_caller = deps.api.addr_validate(&hook_caller)?;

        // add new to registered hooks
        subscribe_denom_to_hook(deps.storage, &denom, reward_state.hook_caller.clone())?;
    }

    if let Some(withdraw_destination) = withdraw_destination {
        reward_state.withdraw_destination = deps.api.addr_validate(&withdraw_destination)?;
    }

    DENOM_REWARD_STATES.save(deps.storage, denom.clone(), &reward_state)?;

    Ok(Response::new()
        .add_attribute("action", "update_denom")
        .add_attribute("denom", denom))
}

/// withdraws the undistributed rewards for a denom. members can claim whatever
/// they earned until this point. this is effectively an inverse to fund and
/// does not affect any already-distributed rewards. can only be called by the
/// admin and only during the distribution period. updates the period finish
/// expiration to the current block.
fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    denom: String,
) -> Result<Response, ContractError> {
    // only the owner can initiate a withdraw
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.to_string())?;

    // withdraw is only possible during the distribution period
    ensure!(
        !reward_state.active_epoch.ends_at.is_expired(&env.block),
        ContractError::RewardsAlreadyDistributed {}
    );

    // withdraw completes the epoch
    reward_state.active_epoch.ends_at = match reward_state.active_epoch.emission_rate.duration {
        Duration::Height(_) => Expiration::AtHeight(env.block.height),
        Duration::Time(_) => Expiration::AtTime(env.block.time),
    };

    // get total rewards distributed based on newly updated ends_at
    let rewards_distributed = reward_state.active_epoch.get_total_rewards()?;

    let clawback_amount = reward_state.funded_amount - rewards_distributed;

    // remove withdrawn funds from amount funded since they are no longer funded
    reward_state.funded_amount = rewards_distributed;

    let clawback_msg = get_transfer_msg(
        reward_state.withdraw_destination.clone(),
        clawback_amount,
        reward_state.denom.clone(),
    )?;

    DENOM_REWARD_STATES.save(deps.storage, denom.to_string(), &reward_state)?;

    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_message(clawback_msg))
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // verify msg
    let _msg: ReceiveMsg = from_json(&wrapper.msg)?;

    let reward_denom_state = DENOM_REWARD_STATES.load(deps.storage, info.sender.to_string())?;
    execute_fund(deps, env, reward_denom_state, wrapper.amount)
}

fn execute_fund_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let fund_coin = one_coin(&info).map_err(|_| ContractError::InvalidFunds {})?;

    let reward_denom_state = DENOM_REWARD_STATES.load(deps.storage, fund_coin.denom.clone())?;

    execute_fund(deps, env, reward_denom_state, fund_coin.amount)
}

fn execute_fund(
    deps: DepsMut,
    env: Env,
    mut denom_reward_state: DenomRewardState,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // we derive the period for which the rewards are funded
    // by looking at the existing reward emission rate and the funded amount
    let funded_period_duration = denom_reward_state
        .active_epoch
        .emission_rate
        .get_funded_period_duration(amount)?;
    let funded_period_value = get_duration_scalar(&funded_period_duration);

    denom_reward_state.bump_last_update(&env.block);
    denom_reward_state.bump_funding_date(&env.block);

    // the duration of rewards period is extended in different ways,
    // depending on the current expiration state and current block
    denom_reward_state.active_epoch.ends_at = match denom_reward_state.active_epoch.ends_at {
        // if this is the first funding of the denom, the new expiration is the
        // funded period duration from the current block
        Expiration::Never {} => funded_period_duration.after(&env.block),
        // otherwise we add the duration units to the existing expiration
        Expiration::AtHeight(h) => {
            if h <= env.block.height {
                // expiration is the funded duration after current block
                Expiration::AtHeight(env.block.height + funded_period_value)
            } else {
                // if the previous expiration had not yet expired, we extend
                // the current rewards period by the newly funded duration
                Expiration::AtHeight(h + funded_period_value)
            }
        }
        Expiration::AtTime(t) => {
            if t <= env.block.time {
                // expiration is the funded duration after current block time
                Expiration::AtTime(env.block.time.plus_seconds(funded_period_value))
            } else {
                // if the previous expiration had not yet expired, we extend
                // the current rewards period by the newly funded duration
                Expiration::AtTime(t.plus_seconds(funded_period_value))
            }
        }
    };
    denom_reward_state.funded_amount += amount;

    DENOM_REWARD_STATES.save(
        deps.storage,
        denom_reward_state.to_str_denom(),
        &denom_reward_state,
    )?;

    Ok(Response::default()
        .add_attribute("action", "fund")
        .add_attribute("fund_denom", denom_reward_state.to_str_denom())
        .add_attribute("fund_amount", amount))
}

fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    // update the rewards information for the sender. this updates the denom reward state
    // and the user reward state, so we operate on the correct state.
    update_rewards(&mut deps, &env, &info.sender, denom.to_string())?;

    // load the updated states. previous `update_rewards` call ensures that these states exist.
    let denom_reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.to_string())?;
    let mut user_reward_state = USER_REWARD_STATES.load(deps.storage, info.sender.clone())?;

    // updating the map returns the previous value if it existed.
    // we set the value to zero and get the amount of pending rewards until this point.
    let claim_amount = user_reward_state
        .pending_denom_rewards
        .insert(denom.to_string(), Uint128::zero())
        .unwrap_or_default();

    // if there are no rewards to claim, error out
    if claim_amount.is_zero() {
        return Err(ContractError::NoRewardsClaimable {});
    }

    // otherwise reflect the updated user reward state and transfer out the claimed rewards
    USER_REWARD_STATES.save(deps.storage, info.sender.clone(), &user_reward_state)?;

    Ok(Response::new()
        .add_message(get_transfer_msg(
            info.sender.clone(),
            claim_amount,
            denom_reward_state.denom,
        )?)
        .add_attribute("action", "claim"))
}

fn execute_update_owner(
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_json_binary(&query_info(deps)?)?),
        QueryMsg::RewardsState {} => Ok(to_json_binary(&query_rewards_state(deps, env)?)?),
        QueryMsg::GetPendingRewards { address } => {
            Ok(to_json_binary(&query_pending_rewards(deps, env, address)?)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::DenomRewardState { denom } => {
            let state = DENOM_REWARD_STATES.load(deps.storage, denom)?;
            Ok(to_json_binary(&state)?)
        }
    }
}

fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

fn query_rewards_state(deps: Deps, _env: Env) -> StdResult<RewardsStateResponse> {
    let rewards = DENOM_REWARD_STATES
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_, v)| v))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(RewardsStateResponse { rewards })
}

/// returns the pending rewards for a given address that are ready to be claimed.
fn query_pending_rewards(deps: Deps, env: Env, addr: String) -> StdResult<PendingRewardsResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    // user may not have interacted with the contract before this query so we
    // potentially return the default user reward state
    let user_reward_state = USER_REWARD_STATES
        .load(deps.storage, addr.clone())
        .unwrap_or_default();
    let reward_states = DENOM_REWARD_STATES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let mut pending_rewards: HashMap<String, Uint128> = HashMap::new();

    // we iterate over every registered denom and calculate the pending rewards for the user
    for (denom, reward_state) in reward_states {
        // first we get the active epoch earned puvp value
        let active_total_earned_puvp =
            get_active_total_earned_puvp(deps, &env.block, &reward_state)?;

        // then we add that to the historical rewards earned puvp
        let total_earned_puvp =
            active_total_earned_puvp.checked_add(reward_state.historical_earned_puvp)?;

        let earned_rewards = get_accrued_rewards_since_last_user_action(
            deps,
            &env,
            &addr,
            total_earned_puvp,
            &reward_state.vp_contract,
            denom.to_string(),
            &user_reward_state,
        )?;
        let existing_amount = user_reward_state
            .pending_denom_rewards
            .get(&denom)
            .cloned()
            .unwrap_or_default();
        pending_rewards.insert(denom, earned_rewards.amount + existing_amount);
    }

    let pending_rewards_response = PendingRewardsResponse {
        address: addr.to_string(),
        pending_rewards,
    };
    Ok(pending_rewards_response)
}
