use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Deps, DepsMut, Env, StdResult, Uint128, Uint256};

use crate::{
    helpers::{
        get_duration_scalar, get_prev_block_total_vp, get_start_end_diff,
        get_voting_power_at_block, scale_factor,
    },
    state::{DenomRewardState, UserRewardState, DENOM_REWARD_STATES, USER_REWARD_STATES},
};

/// updates the user reward state for a given denom and user address.
/// also syncs the global denom reward state config with the latest puvp values.
pub fn update_rewards(deps: &mut DepsMut, env: &Env, addr: &Addr, denom: String) -> StdResult<()> {
    // user may not have a reward state set yet if that is their first time claiming,
    // so we default to an empty state
    let mut user_reward_state = USER_REWARD_STATES
        .may_load(deps.storage, addr.clone())?
        .unwrap_or_default();
    let mut denom_reward_state = DENOM_REWARD_STATES.load(deps.storage, denom.clone())?;

    // first we go over the historic epochs and sum the historic rewards earned
    let total_historic_puvp = denom_reward_state.get_historic_rewards_earned_puvp_sum();

    // we update the active epoch earned puvp value up to the current block
    denom_reward_state.active_epoch.total_earned_puvp =
        get_active_total_earned_puvp(deps.as_ref(), &env.block, &denom_reward_state)?;
    denom_reward_state.bump_last_update(&env.block);

    // the total applicable puvp is the sum of all historic puvp and the active epoch puvp
    let total_applicable_puvp = denom_reward_state
        .active_epoch
        .total_earned_puvp
        .checked_add(total_historic_puvp)?;

    let earned_rewards = get_accrued_rewards_since_last_user_action(
        deps.as_ref(),
        env,
        addr,
        total_applicable_puvp,
        &denom_reward_state.vp_contract,
        denom.to_string(),
        &user_reward_state,
    )?;

    // get the pre-existing pending reward amount for the denom
    let previous_pending_denom_reward_amount = user_reward_state
        .pending_denom_rewards
        .get(&denom)
        .cloned()
        .unwrap_or_default();

    let amount_sum = earned_rewards
        .amount
        .checked_add(previous_pending_denom_reward_amount)?;

    // get the amount of newly earned rewards for the denom
    user_reward_state
        .pending_denom_rewards
        .insert(denom.clone(), amount_sum);

    // update the accounted for amount to that of the total applicable puvp
    user_reward_state
        .accounted_denom_rewards_puvp
        .insert(denom.clone(), total_applicable_puvp);

    // reflect the updated state changes
    USER_REWARD_STATES.save(deps.storage, addr.clone(), &user_reward_state)?;
    DENOM_REWARD_STATES.save(deps.storage, denom.clone(), &denom_reward_state)?;

    Ok(())
}

/// Calculate the total rewards earned per unit voting power in the active epoch
/// since the last update.
pub fn get_active_total_earned_puvp(
    deps: Deps,
    block: &BlockInfo,
    reward_state: &DenomRewardState,
) -> StdResult<Uint256> {
    let curr = reward_state.active_epoch.total_earned_puvp;

    let prev_total_power = get_prev_block_total_vp(deps, block, &reward_state.vp_contract)?;

    let last_time_rewards_distributed = reward_state.get_latest_reward_distribution_time(block);

    // get the duration from the last time rewards were updated to the last time
    // rewards were distributed. this will be 0 if the rewards were updated at
    // or after the last time rewards were distributed.
    let new_reward_distribution_duration: Uint128 =
        get_start_end_diff(&last_time_rewards_distributed, &reward_state.last_update)?.into();

    if prev_total_power.is_zero() {
        Ok(curr)
    } else {
        // count intervals of the rewards emission that have passed since the
        // last update which need to be distributed
        let complete_distribution_periods = new_reward_distribution_duration.checked_div(
            get_duration_scalar(&reward_state.active_epoch.emission_rate.duration).into(),
        )?;
        // It is impossible for this to overflow as total rewards can never
        // exceed max value of Uint128 as total tokens in existence cannot
        // exceed Uint128 (because the bank module Coin type uses Uint128).
        let new_rewards_distributed = reward_state
            .active_epoch
            .emission_rate
            .amount
            .full_mul(complete_distribution_periods)
            .checked_mul(scale_factor())?;

        // the new rewards per unit voting power that have been distributed
        // since the last update
        let new_rewards_puvp = new_rewards_distributed.checked_div(prev_total_power.into())?;
        Ok(curr.checked_add(new_rewards_puvp)?)
    }
}

// get a user's rewards not yet accounted for in their reward states.
pub fn get_accrued_rewards_since_last_user_action(
    deps: Deps,
    env: &Env,
    addr: &Addr,
    total_earned_puvp: Uint256,
    vp_contract: &Addr,
    denom: String,
    user_reward_state: &UserRewardState,
) -> StdResult<Coin> {
    // get the user's voting power at the current height
    let voting_power: Uint256 =
        get_voting_power_at_block(deps, &env.block, vp_contract, addr)?.into();

    // get previous reward per unit voting power accounted for
    let user_last_reward_puvp = user_reward_state
        .accounted_denom_rewards_puvp
        .get(&denom)
        .cloned()
        .unwrap_or_default();

    // calculate the difference between the current total reward per unit
    // voting power distributed and the user's latest reward per unit voting
    // power accounted for.
    let reward_factor = total_earned_puvp.checked_sub(user_last_reward_puvp)?;

    // calculate the amount of rewards earned:
    // voting_power * reward_factor / scale_factor
    let accrued_rewards_amount: Uint128 = voting_power
        .checked_mul(reward_factor)?
        .checked_div(scale_factor())?
        .try_into()?;

    Ok(coin(accrued_rewards_amount.u128(), denom))
}
