#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, ensure, from_json, to_json_binary, Addr, BankMsg, Binary, BlockInfo, CosmosMsg, Decimal,
    Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult, Uint128, Uint256,
    WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ReceiveMsg, Denom};
use cw_utils::{one_coin, Duration, Expiration};
use dao_interface::voting::{
    InfoResponse, Query as VotingQueryMsg, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use std::cmp::min;
use std::collections::HashMap;
use std::convert::TryInto;

use crate::hooks::{
    execute_membership_changed, execute_nft_stake_changed, execute_stake_changed,
    subscribe_denom_to_hook,
};
use crate::msg::{
    ExecuteMsg, InstantiateMsg, PendingRewardsResponse, QueryMsg, ReceiveMsg,
    RegisterRewardDenomMsg, RewardEmissionRate, RewardsStateResponse,
};
use crate::state::{DenomRewardState, EpochConfig, DENOM_REWARD_STATES, USER_REWARD_STATES};
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
        ExecuteMsg::Claim { denom } => execute_claim(deps, env, info, denom),
        ExecuteMsg::Fund {} => execute_fund_native(deps, env, info),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::Shutdown { denom } => execute_shutdown(deps, info, env, denom),
        ExecuteMsg::RegisterRewardDenom(register_msg) => {
            execute_register_reward_denom(deps, info, register_msg)
        }
        ExecuteMsg::UpdateRewardEmissionRate {
            denom,
            emission_rate,
        } => execute_update_reward_rate(deps, env, info, denom, emission_rate),
    }
}

/// updates the reward emission rate for a registered denom
fn execute_update_reward_rate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    new_emission_rate: RewardEmissionRate,
) -> Result<Response, ContractError> {
    // only the owner can update the reward rate
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.clone())?;
    reward_state.active_epoch_config.total_earned_puvp = get_total_earned_puvp(
        deps.as_ref(),
        &env.block,
        &reward_state.active_epoch_config,
        &reward_state.vp_contract,
        &reward_state.last_update,
    )?;
    reward_state = reward_state.bump_last_update(&env.block);

    // transition the epoch to the new emission rate and save
    reward_state.transition_epoch(new_emission_rate, &env.block)?;

    DENOM_REWARD_STATES.save(deps.storage, denom.clone(), &reward_state)?;

    Ok(Response::new().add_attribute("action", "update_reward_rate"))
}

/// registers a new denom for rewards distribution.
/// only the owner can register a new denom.
/// a denom can only be registered once; update if you need to change something.
fn execute_register_reward_denom(
    deps: DepsMut,
    info: MessageInfo,
    msg: RegisterRewardDenomMsg,
) -> Result<Response, ContractError> {
    // only the owner can register a new denom
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    msg.emission_rate.validate_emission_time_window()?;

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
        active_epoch_config: EpochConfig {
            started_at: Expiration::Never {},
            ends_at: Expiration::Never {},
            emission_rate: msg.emission_rate,
            total_earned_puvp: Uint256::zero(),
            finish_height: None,
        },
        last_update: Expiration::Never {},
        vp_contract,
        hook_caller: hook_caller.clone(),
        funded_amount: Uint128::zero(),
        withdraw_destination,
        historic_epoch_configs: vec![],
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
    subscribe_denom_to_hook(deps, str_denom, hook_caller.clone())?;

    Ok(Response::default())
}

/// shutdown the rewards distributor contract.
/// can only be called by the admin and only during the distribution period.
/// this will clawback all (undistributed) future rewards to the admin.
/// updates the period finish expiration to the current block.
fn execute_shutdown(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    denom: String,
) -> Result<Response, ContractError> {
    // only the owner can initiate a shutdown
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.to_string())?;

    // shutdown is only possible during the distribution period
    ensure!(
        !reward_state
            .active_epoch_config
            .ends_at
            .is_expired(&env.block),
        ContractError::ShutdownError("Reward period already finished".to_string())
    );

    // we get the start and end scalar values in u64 (seconds/blocks)
    let started_at = reward_state.get_started_at_scalar()?;
    let ends_at = reward_state.get_ends_at_scalar()?;
    let reward_duration = ends_at - started_at;

    // find the % of reward_duration that remains from current block
    let passed_units_since_start = match reward_state.active_epoch_config.emission_rate.duration {
        Duration::Height(_) => Uint128::from(env.block.height - started_at),
        Duration::Time(_) => Uint128::from(env.block.time.seconds() - started_at),
    };

    // get the fraction of what part of rewards duration is in the past
    // and sub from 1 to get the remaining rewards
    let remaining_reward_duration_fraction = Decimal::one()
        .checked_sub(Decimal::from_ratio(
            passed_units_since_start,
            reward_duration,
        ))
        .map_err(|e| ContractError::Std(StdError::overflow(e)))?;

    // to get the clawback msg
    let clawback_msg = get_transfer_msg(
        reward_state.withdraw_destination.clone(),
        reward_state.funded_amount * remaining_reward_duration_fraction,
        reward_state.denom.clone(),
    )?;

    // shutdown completes the rewards
    reward_state.active_epoch_config.ends_at =
        match reward_state.active_epoch_config.emission_rate.duration {
            Duration::Height(_) => Expiration::AtHeight(env.block.height),
            Duration::Time(_) => Expiration::AtTime(env.block.time),
        };

    DENOM_REWARD_STATES.save(deps.storage, denom.to_string(), &reward_state)?;

    Ok(Response::new()
        .add_attribute("action", "shutdown")
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
        .active_epoch_config
        .emission_rate
        .get_funded_period_duration(amount)?;
    let funded_period_value = get_duration_scalar(&funded_period_duration);

    denom_reward_state = denom_reward_state
        .bump_funding_date(&env.block)
        .bump_last_update(&env.block);

    // the duration of rewards period is extended in different ways,
    // depending on the current expiration state and current block
    denom_reward_state.active_epoch_config.ends_at =
        match denom_reward_state.active_epoch_config.ends_at {
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

    Ok(Response::default())
}

fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    // update the rewards information for the sender.
    update_rewards(&mut deps, &env, &info.sender, denom.to_string())?;

    // get the denom state for the string-based denom
    let denom_reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.to_string())?;

    let mut amount = Uint128::zero();

    let mut user_reward_state = USER_REWARD_STATES
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or_default();
    println!("user reward state during claim: {:?}", user_reward_state);

    // updating the map returns the previous value if it existed.
    // we set the value to zero and store it in the amount defined before the update.
    amount = user_reward_state
        .pending_denom_rewards
        .insert(denom.to_string(), Uint128::zero())
        .unwrap_or_default();
    println!("amount: {:?}", amount);
    // if active epoch never expires, means rewards are paused. we manually query the rewards.
    if let Expiration::Never {} = denom_reward_state.active_epoch_config.ends_at {
        let pending_rewards = query_pending_rewards(deps.as_ref(), env, info.sender.to_string())?;
        println!("pending rewards: {:?}", pending_rewards);
        amount = *pending_rewards
            .pending_rewards
            .get(&denom)
            .unwrap_or(&Uint128::zero());
        let current_denom_reward_puvp = user_reward_state
            .denom_rewards_puvp
            .get(&denom)
            .cloned()
            .unwrap_or_default();

        user_reward_state.denom_rewards_puvp.insert(
            denom.clone(),
            current_denom_reward_puvp
                .checked_add(Uint256::from(amount).checked_mul(scale_factor()).unwrap())
                .unwrap(),
        );
    }

    if amount.is_zero() {
        return Err(ContractError::NoRewardsClaimable {});
    }

    USER_REWARD_STATES.save(deps.storage, info.sender.clone(), &user_reward_state)?;
    Ok(Response::new()
        .add_message(get_transfer_msg(
            info.sender.clone(),
            amount,
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

pub fn update_rewards(deps: &mut DepsMut, env: &Env, addr: &Addr, denom: String) -> StdResult<()> {
    let reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.clone())?;

    // first, we calculate the latest total rewards per unit voting power
    // and update them
    let total_earned_puvp = get_total_earned_puvp(
        deps.as_ref(),
        &env.block,
        &reward_state.active_epoch_config,
        &reward_state.vp_contract,
        &reward_state.last_update,
    )?;

    // update the denom state's total rewards earned and last updated
    DENOM_REWARD_STATES.update(deps.storage, denom.clone(), |state| -> StdResult<_> {
        match state {
            Some(mut rc) => {
                rc.active_epoch_config.total_earned_puvp = total_earned_puvp;
                Ok(rc.bump_last_update(&env.block))
            }
            None => Err(StdError::generic_err("Denom reward state not found")),
        }
    })?;

    // then we calculate the rewards earned since last user action
    println!("getting earned rewards for user...");
    let earned_rewards = if total_earned_puvp.is_zero() {
        println!("total earned puvp is zero, returning zero rewards");
        HashMap::from_iter(vec![(denom.clone(), Uint128::zero())])
    } else {
        get_accrued_rewards_since_last_user_action(
            deps.as_ref(),
            env,
            addr,
            total_earned_puvp,
            &reward_state.vp_contract,
            denom.clone(),
        )?
    };

    // reflect the earned rewards in the user's reward state
    USER_REWARD_STATES.update(deps.storage, addr.clone(), |state| -> StdResult<_> {
        // if user does not yet have state, create a new one
        let mut user_reward_state = state.unwrap_or_default();

        // get the pre-existing pending reward amount for the denom
        let previous_pending_denom_reward_amount = *user_reward_state
            .pending_denom_rewards
            .get(&denom)
            .unwrap_or(&Uint128::zero());

        // get the amount of newly earned rewards for the denom
        let earned_rewards_amount = earned_rewards.get(&denom).cloned().unwrap_or_default();

        user_reward_state.pending_denom_rewards.insert(
            denom.clone(),
            previous_pending_denom_reward_amount + earned_rewards_amount,
        );

        // update the user's earned rewards that have been accounted for
        user_reward_state
            .denom_rewards_puvp
            .insert(denom.clone(), total_earned_puvp);

        Ok(user_reward_state)
    })?;
    Ok(())
}

/// Calculate the total rewards earned per unit voting power since the last
/// update.
fn get_total_earned_puvp(
    deps: Deps,
    block: &BlockInfo,
    epoch: &EpochConfig,
    vp_contract: &Addr,
    last_update: &Expiration,
) -> StdResult<Uint256> {
    let curr = epoch.total_earned_puvp;
    println!("[EPOCH EARNED PUVP] epoch.total_earned_puvp: \t\t{curr}",);
    // query the total voting power just before this block from the voting power
    // contract
    let prev_total_power = get_prev_block_total_vp(deps, block, vp_contract)?;
    println!("[EPOCH EARNED PUVP] prev_total_power: \t\t\t{prev_total_power}",);
    // if epoch is past, we return the epoch end date. otherwise the specified block.
    // returns time scalar based on the epoch date config.
    let last_time_rewards_distributed = match epoch.ends_at {
        Expiration::Never {} => *last_update,
        Expiration::AtHeight(h) => Expiration::AtHeight(min(block.height, h)),
        Expiration::AtTime(t) => Expiration::AtTime(min(block.time, t)),
    };

    println!(
        "[EPOCH EARNED PUVP] last_time_rewards_distributed: \t{last_time_rewards_distributed}",
    );
    // get the duration from the last time rewards were updated to the last time
    // rewards were distributed. this will be 0 if the rewards were updated at
    // or after the last time rewards were distributed.
    let new_reward_distribution_duration: Uint128 =
        get_start_end_diff(&last_time_rewards_distributed, last_update)?.into();

    println!(
        "[EPOCH EARNED PUVP] new_reward_distribution_duration: \t{new_reward_distribution_duration}"
    );
    if prev_total_power.is_zero() {
        Ok(curr)
    } else {
        let duration_value = get_duration_scalar(&epoch.emission_rate.duration);

        println!("[EPOCH EARNED PUVP] duration_value: \t\t\t{duration_value}",);
        // count intervals of the rewards emission that have passed since the
        // last update which need to be distributed
        let complete_distribution_periods =
            new_reward_distribution_duration.checked_div(Uint128::from(duration_value))?;

        println!(
            "[EPOCH EARNED PUVP] complete_distribution_periods: \t{complete_distribution_periods}",
        );
        // It is impossible for this to overflow as total rewards can never
        // exceed max value of Uint128 as total tokens in existence cannot
        // exceed Uint128 (because the bank module Coin type uses Uint128).
        let new_rewards_distributed = epoch
            .emission_rate
            .amount
            .full_mul(complete_distribution_periods)
            .checked_mul(scale_factor())?;

        println!("[EPOCH EARNED PUVP] new_rewards_distributed:\t\t{new_rewards_distributed}",);
        // the new rewards per unit voting power that have been distributed
        // since the last update
        let new_rewards_puvp = new_rewards_distributed.checked_div(prev_total_power.into())?;
        println!("[EPOCH EARNED PUVP] new_rewards_puvp:\t\t\t{new_rewards_puvp}",);
        Ok(curr + new_rewards_puvp)
    }
}

// get a user's rewards not yet accounted for in their reward state
fn get_accrued_rewards_since_last_user_action(
    deps: Deps,
    env: &Env,
    addr: &Addr,
    total_earned_puvp: Uint256,
    vp_contract: &Addr,
    denom: String,
) -> StdResult<HashMap<String, Uint128>> {
    let user_reward_state = USER_REWARD_STATES
        .load(deps.storage, addr.clone())
        .unwrap_or_default();

    // get previous reward per unit voting power accounted for
    let user_last_reward_puvp = user_reward_state
        .denom_rewards_puvp
        .get(&denom)
        .cloned()
        .unwrap_or_default();

    println!("[REWARDS ACCRUED SINCE LAST USER ACTION]addr:\t\t{addr}");
    println!(
        "[REWARDS ACCRUED SINCE LAST USER ACTION]user_last_reward_puvp:\t\t{user_last_reward_puvp}",
    );
    println!("[REWARDS ACCRUED SINCE LAST USER ACTION]total_earned_puvp:\t\t{total_earned_puvp}",);
    // calculate the difference between the current total reward per unit
    // voting power distributed and the user's latest reward per unit voting
    // power accounted for.
    // TODO: this may overlap with a historic epoch in the past, this needs to be checked for.
    let reward_factor = total_earned_puvp.checked_sub(user_last_reward_puvp)?;
    println!("[REWARDS ACCRUED SINCE LAST USER ACTION]reward_factor:\t\t\t{reward_factor}",);
    // get the user's voting power at the current height
    let voting_power = Uint256::from(get_voting_power_at_block(
        deps,
        &env.block,
        vp_contract,
        addr,
    )?);
    println!("[REWARDS ACCRUED SINCE LAST USER ACTION]voting power at now:\t\t{reward_factor}",);

    // calculate the amount of rewards earned:
    // voting_power * reward_factor / scale_factor
    let accrued_rewards_amount: Uint128 = voting_power
        .checked_mul(reward_factor)?
        .checked_div(scale_factor())?
        .try_into()?;
    println!(
        "[REWARDS ACCRUED SINCE LAST USER ACTION]accrued_rewards_amount:\t\t{accrued_rewards_amount}"
    );

    Ok(HashMap::from_iter(vec![(
        denom.to_string(),
        accrued_rewards_amount,
    )]))
}

fn get_prev_block_total_vp(
    deps: Deps,
    block: &BlockInfo,
    contract_addr: &Addr,
) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::TotalPowerAtHeight {
        height: Some(block.height.checked_sub(1).unwrap_or_default()),
    };
    let resp: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

fn get_voting_power_at_block(
    deps: Deps,
    block: &BlockInfo,
    contract_addr: &Addr,
    addr: &Addr,
) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::VotingPowerAtHeight {
        address: addr.into(),
        height: Some(block.height),
    };
    let resp: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

/// returns underlying scalar value for a given duration.
/// if the duration is in blocks, returns the block height.
/// if the duration is in time, returns the time in seconds.
fn get_duration_scalar(duration: &Duration) -> u64 {
    match duration {
        Duration::Height(h) => *h,
        Duration::Time(t) => *t,
    }
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

fn query_pending_rewards(deps: Deps, env: Env, addr: String) -> StdResult<PendingRewardsResponse> {
    let addr = deps.api.addr_validate(&addr)?;

    let user_reward_state = USER_REWARD_STATES
        .load(deps.storage, addr.clone())
        .unwrap_or_default();
    let reward_states = DENOM_REWARD_STATES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let default_amt = Uint128::zero();

    let mut pending_rewards: HashMap<String, Uint128> = HashMap::new();

    // we iterate over every registered denom and calculate the pending rewards for the user
    for (denom, reward_state) in reward_states {
        // first we go over the historic epochs and sum the historic puvp
        let total_historic_puvp: Uint256 = reward_state
            .historic_epoch_configs
            .iter()
            .map(|e| e.total_earned_puvp)
            .sum();

        // then we get the active epoch earned puvp value
        let total_earned_puvp = get_total_earned_puvp(
            deps,
            &env.block,
            &reward_state.active_epoch_config,
            &reward_state.vp_contract,
            &reward_state.last_update,
        )?;
        println!("[QUERY PENDING REWARDS] total historic puvp:\t\t{total_historic_puvp}",);
        println!("[QUERY PENDING REWARDS] active epoch earned puvp:\t{total_earned_puvp}",);
        let earned_rewards = get_accrued_rewards_since_last_user_action(
            deps,
            &env,
            &addr,
            total_earned_puvp.checked_add(total_historic_puvp)?,
            &reward_state.vp_contract,
            denom.to_string(),
        )?;
        let earned_amount = earned_rewards.get(&denom).unwrap_or(&default_amt);
        println!("[QUERY PENDING REWARDS] earned amount: {earned_amount}");
        let existing_amount = user_reward_state
            .pending_denom_rewards
            .get(&denom)
            .unwrap_or(&default_amt);
        println!("[QUERY PENDING REWARDS] existing amount: {existing_amount}",);
        pending_rewards.insert(denom, *earned_amount + *existing_amount);
    }

    let pending_rewards_response = PendingRewardsResponse {
        address: addr.to_string(),
        pending_rewards,
    };
    Ok(pending_rewards_response)
}

/// Returns the appropriate CosmosMsg for transferring the reward token.
fn get_transfer_msg(recipient: Addr, amount: Uint128, denom: Denom) -> StdResult<CosmosMsg> {
    match denom {
        Denom::Native(denom) => Ok(BankMsg::Send {
            to_address: recipient.into_string(),
            amount: coins(amount.u128(), denom),
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

pub(crate) fn scale_factor() -> Uint256 {
    Uint256::from(10u8).pow(39)
}

/// Calculate the duration from start to end. If the end is at or before the
/// start, return 0.
pub fn get_start_end_diff(end: &Expiration, start: &Expiration) -> StdResult<u64> {
    match (end, start) {
        (Expiration::AtHeight(end), Expiration::AtHeight(start)) => {
            if end > start {
                Ok(end - start)
            } else {
                Ok(0)
            }
        }
        (Expiration::AtTime(end), Expiration::AtTime(start)) => {
            if end > start {
                Ok(end.seconds() - start.seconds())
            } else {
                Ok(0)
            }
        }
        (Expiration::Never {}, Expiration::Never {}) => Ok(0),
        _ => Err(StdError::generic_err(format!(
            "incompatible expirations: got end {:?}, start {:?}",
            end, start
        ))),
    }
}

fn validate_voting_power_contract(
    deps: &DepsMut,
    vp_contract: String,
) -> Result<Addr, ContractError> {
    let vp_contract = deps.api.addr_validate(&vp_contract)?;
    let _: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &vp_contract,
        &VotingQueryMsg::TotalPowerAtHeight { height: None },
    )?;
    Ok(vp_contract)
}
