use cosmwasm_std::{Addr, DepsMut, Env, StdResult};

use crate::{
    contract::{get_accrued_rewards_since_last_user_action, get_total_earned_puvp},
    state::{DENOM_REWARD_STATES, USER_REWARD_STATES},
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

    // we update the active epoch earned puvp value, from start to the current block
    denom_reward_state.active_epoch_config.total_earned_puvp =
        get_total_earned_puvp(deps.as_ref(), &env.block, &denom_reward_state)?;
    denom_reward_state.bump_last_update(&env.block);

    // the total applicable puvp is the sum of all historic puvp and the active epoch puvp
    let total_applicable_puvp = denom_reward_state
        .active_epoch_config
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

    let earned_amount = earned_rewards.amount;

    // get the pre-existing pending reward amount for the denom
    let previous_pending_denom_reward_amount = user_reward_state
        .pending_denom_rewards
        .get(&denom)
        .cloned()
        .unwrap_or_default();

    let amount_sum = earned_amount.checked_add(previous_pending_denom_reward_amount)?;

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
