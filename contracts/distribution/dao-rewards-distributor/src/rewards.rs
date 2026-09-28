use std::cmp::min;

use cosmwasm_std::{Addr, BlockInfo, Decimal, Deps, DepsMut, Env, StdResult, Uint128, Uint256};
use cw20::Expiration;

use crate::{
    helpers::{
        get_total_voting_power_at_block, get_voting_power_at_block, get_voting_power_at_height,
        scale_factor, DurationExt, ExpirationExt,
    },
    state::{DistributionState, EmissionRate, UserRewardState, DISTRIBUTIONS, USER_REWARDS},
    ContractError,
};

/// updates the user reward state for a given distribution and user address.
/// also syncs the global reward state with the latest puvp values.
pub fn update_rewards(
    deps: &mut DepsMut,
    env: &Env,
    addr: &Addr,
    distribution_id: u64,
) -> Result<(), ContractError> {
    let mut distribution = DISTRIBUTIONS
        .load(deps.storage, distribution_id)
        .map_err(|_| ContractError::DistributionNotFound {
            id: distribution_id,
        })?;

    // user may not have a reward state set yet if that is their first time
    // claiming, so we default to an empty state
    let mut user_reward_state = USER_REWARDS
        .may_load(deps.storage, addr.clone())?
        .unwrap_or_default();

    // first update the active epoch earned puvp value up to the current block
    distribution.active_epoch.total_earned_puvp =
        get_active_total_earned_puvp(deps.as_ref(), &env.block, &distribution)?;
    distribution.active_epoch.bump_last_updated(&env.block);

    // then calculate the total applicable puvp, which is the sum of historical
    // rewards earned puvp and the active epoch total earned puvp we just
    // updated above based on the current block
    let total_applicable_puvp = distribution
        .active_epoch
        .total_earned_puvp
        .checked_add(distribution.historical_earned_puvp)?;

    let unaccounted_for_rewards = get_accrued_rewards_not_yet_accounted_for(
        deps.as_ref(),
        env,
        addr,
        total_applicable_puvp,
        &distribution,
        &user_reward_state,
    )?;

    // get the pre-existing pending reward amount for the distribution
    let previous_pending_reward_amount = user_reward_state
        .pending_rewards
        .get(&distribution.id)
        .cloned()
        .unwrap_or_default();

    let amount_sum = unaccounted_for_rewards.checked_add(previous_pending_reward_amount)?;

    // get the amount of newly earned rewards for the distribution
    user_reward_state
        .pending_rewards
        .insert(distribution_id, amount_sum);

    // update the accounted for amount to that of the total applicable puvp
    user_reward_state
        .accounted_for_rewards_puvp
        .insert(distribution_id, total_applicable_puvp);

    // record the height of this checkpoint so that future accruals can
    // conservatively bound the voting power used in case a voting power
    // change hook is missed before the next checkpoint. see
    // `get_accrued_rewards_not_yet_accounted_for`.
    user_reward_state
        .last_updated_height
        .insert(distribution_id, env.block.height);

    // reflect the updated state changes
    USER_REWARDS.save(deps.storage, addr.clone(), &user_reward_state)?;
    DISTRIBUTIONS.save(deps.storage, distribution_id, &distribution)?;

    Ok(())
}

/// Calculate the total rewards per unit voting power in the active epoch.
pub fn get_active_total_earned_puvp(
    deps: Deps,
    block: &BlockInfo,
    distribution: &DistributionState,
) -> Result<Uint256, ContractError> {
    match distribution.active_epoch.emission_rate {
        EmissionRate::Paused {} => Ok(Uint256::zero()),
        // this is updated manually during funding, so just return it here.
        EmissionRate::Immediate {} => Ok(distribution.active_epoch.total_earned_puvp),
        EmissionRate::Linear {
            amount, duration, ..
        } => {
            let curr = distribution.active_epoch.total_earned_puvp;

            let last_time_rewards_distributed =
                distribution.get_latest_reward_distribution_time(block);

            // if never distributed rewards (i.e. not yet funded), return
            // current, which must be 0.
            if let Expiration::Never {} = last_time_rewards_distributed {
                return Ok(curr);
            }

            // get the duration from the last time rewards were updated to the
            // last time rewards were distributed. this will be 0 if the rewards
            // were updated at or after the last time rewards were distributed.
            let new_reward_distribution_duration = last_time_rewards_distributed
                .duration_since(&distribution.active_epoch.last_updated_total_earned_puvp)?;

            // no need to query total voting power and do math if distribution
            // is already up to date.
            if new_reward_distribution_duration.is_zero() {
                return Ok(curr);
            }

            let total_power =
                get_total_voting_power_at_block(deps, block, &distribution.vp_contract)?;

            // if no voting power is registered, no one should receive rewards.
            if total_power.is_zero() {
                Ok(curr)
            } else {
                // count (partial) intervals of the rewards emission that have
                // passed since the last update which need to be distributed
                let complete_distribution_periods =
                    new_reward_distribution_duration.ratio(&duration)?;

                // the new rewards per unit voting power that have been
                // distributed since the last update:
                //
                // amount * scale_factor * periods / total_power
                //
                // the precision scale must be applied before flooring, or
                // small per-update emissions are lost entirely. the product
                // is computed in 512-bit space so that large (valid) emission
                // amounts cannot overflow the intermediate value, and flooring
                // once here is equivalent to flooring the scaled amount and
                // then flooring again on division by total power.
                let new_rewards_puvp = scale_factor().checked_multiply_ratio(
                    Uint256::from(amount)
                        .checked_mul(complete_distribution_periods.atomics().into())?,
                    Uint256::from(Decimal::one().atomics()).checked_mul(total_power.into())?,
                )?;
                Ok(curr.checked_add(new_rewards_puvp)?)
            }
        }
    }
}

// get a user's rewards not yet accounted for in their reward state (not pending
// nor claimed, but available to them due to the passage of time).
//
// this multiplies a voting power by the `reward_factor` (the change in
// rewards earned per unit voting power since the user's rewards were last
// accounted for). that is only correct if the user's voting power was
// constant for the entire period the `reward_factor` covers, which is
// normally guaranteed by voting power change hooks: any stake/unstake (or
// other voting power change) updates the user's reward state (and thus
// resets `reward_factor` to start counting from that height) before the
// voting power actually changes.
//
// if a voting power change hook is ever missed (e.g. because a hook receiver
// is allowed to fail without reverting the underlying stake change), that
// guarantee breaks: the user's voting power may have changed without a
// checkpoint, so naively using their current voting power for the whole
// unaccounted-for period can over-credit them, potentially by more than was
// ever emitted. to guard against this, we conservatively use the minimum of:
//   - the user's voting power at `env.block.height` (the current, existing
//     query -- this reflects the state at the start of the current block,
//     before any changes made in the current block), and
//   - the user's voting power at `last_updated_height + 1`, i.e. the voting
//     power that took effect immediately after their last checkpoint (voting
//     power changes take effect on the following block).
// in the healthy case (no missed hooks) the user's voting power has not
// changed since their last checkpoint, so these two are equal and behavior is
// unchanged. after a single missed stake or unstake, the user is
// under-credited for that period rather than over-credited.
//
// this is only a mitigation, not a complete fix: it only compares the two
// endpoints, so a missed change that moves away from and then back to the
// same voting power within a single unaccounted-for period is not detected.
// the per-distribution `claimable_funds` cap (see `state::DistributionState`)
// exists as defense in depth against that residual risk.
pub fn get_accrued_rewards_not_yet_accounted_for(
    deps: Deps,
    env: &Env,
    addr: &Addr,
    total_earned_puvp: Uint256,
    distribution: &DistributionState,
    user_reward_state: &UserRewardState,
) -> StdResult<Uint128> {
    // get previous reward per unit voting power accounted for
    let user_last_reward_puvp = user_reward_state
        .accounted_for_rewards_puvp
        .get(&distribution.id)
        .cloned()
        .unwrap_or_default();

    // calculate the difference between the current total reward per unit
    // voting power distributed and the user's latest reward per unit voting
    // power accounted for.
    let reward_factor = total_earned_puvp.checked_sub(user_last_reward_puvp)?;

    // nothing new has been earned since the user's last checkpoint, so there
    // is nothing to accrue. skip querying voting power entirely.
    if reward_factor.is_zero() {
        return Ok(Uint128::zero());
    }

    // get the user's voting power at the current height (start of the
    // current block, before any changes made in the current block).
    let voting_power: Uint256 =
        get_voting_power_at_block(deps, &env.block, &distribution.vp_contract, addr)?.into();

    // determine the height right after the user's last checkpoint for this
    // distribution, to use as the conservative voting power reference point.
    // if the user has never been checkpointed for this distribution (their
    // accounted for puvp is the default of 0, i.e. the distribution's
    // start), fall back to the distribution's creation height, since that is
    // the earliest point their voting power could be relevant from.
    // if we don't know either (legacy distribution/user state stored before
    // this field was introduced), we have no reference point to be
    // conservative against, so fall back to the original behavior of only
    // using the current voting power.
    let last_checkpoint_height = match user_reward_state
        .last_updated_height
        .get(&distribution.id)
        .copied()
    {
        Some(height) => Some(height),
        None if !user_reward_state
            .accounted_for_rewards_puvp
            .contains_key(&distribution.id) =>
        {
            distribution.created_at_height
        }
        None => None,
    };

    let voting_power = match last_checkpoint_height {
        Some(last_checkpoint_height) => {
            // voting power changes take effect on the block following the
            // one they occur in, so the voting power right after the user's
            // last checkpoint is the voting power at `last_checkpoint_height
            // + 1`. cap this at the current block height so we never query a
            // future height: if the checkpoint happened this same block (or,
            // defensively, somehow later), there's nothing more to query --
            // the current voting power is already the answer.
            let reference_height = min(last_checkpoint_height.saturating_add(1), env.block.height);

            if reference_height == env.block.height {
                voting_power
            } else {
                let reference_voting_power: Uint256 = get_voting_power_at_height(
                    deps,
                    reference_height,
                    &distribution.vp_contract,
                    addr,
                )?
                .into();

                min(voting_power, reference_voting_power)
            }
        }
        None => voting_power,
    };

    // calculate the amount of rewards earned:
    // voting_power * reward_factor / scale_factor
    let accrued_rewards_amount: Uint128 = voting_power
        .checked_mul(reward_factor)?
        .checked_div(scale_factor())?
        .try_into()?;

    Ok(accrued_rewards_amount)
}
