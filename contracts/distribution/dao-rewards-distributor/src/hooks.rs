use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};

use crate::{
    contract::{get_accrued_rewards_since_last_user_action, get_total_earned_puvp},
    state::{DENOM_REWARD_STATES, REGISTERED_HOOK_DENOMS, USER_REWARD_STATES},
    ContractError,
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

    // first we go over the historic epochs and sum the historic puvp
    let total_historic_puvp = denom_reward_state.get_historic_rewards_earned_puvp_sum();

    // we update the active epoch earned puvp value, from it's start to the current block
    denom_reward_state.active_epoch_config.total_earned_puvp = get_total_earned_puvp(
        deps.as_ref(),
        &env.block,
        &denom_reward_state.active_epoch_config,
        &denom_reward_state.vp_contract,
        &denom_reward_state.last_update,
    )?;

    // the total applicable puvp is the sum of all historic puvp and the active epoch puvp
    let total_applicable_puvp = denom_reward_state
        .active_epoch_config
        .total_earned_puvp
        .checked_add(total_historic_puvp)?;

    denom_reward_state.bump_last_update(&env.block);

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

/// Register a hook caller contract for a given denom.
pub(crate) fn subscribe_denom_to_hook(
    deps: DepsMut,
    denom: String,
    hook: Addr,
) -> Result<(), ContractError> {
    REGISTERED_HOOK_DENOMS.update(deps.storage, hook, |denoms| -> StdResult<_> {
        let mut denoms = denoms.unwrap_or_default();
        denoms.push(denom.to_string());
        Ok(denoms)
    })?;
    Ok(())
}

/// Ensures hooks that update voting power are only called by a designated
/// hook_caller contract.
/// Returns a list of denoms that the hook caller is registered for.
pub(crate) fn get_hook_caller_registered_denoms(
    deps: Deps,
    info: MessageInfo,
) -> Result<Vec<String>, ContractError> {
    // only a designated hook_caller contract can call this hook.
    // failing to load the registered denoms for a given hook returns an error.
    REGISTERED_HOOK_DENOMS
        .load(deps.storage, info.sender.clone())
        .map_err(|_| ContractError::InvalidHookSender {})
}

pub(crate) fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooked_denoms = get_hook_caller_registered_denoms(deps.as_ref(), info)?;

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr, hooked_denoms),
        StakeChangedHookMsg::Unstake { addr, .. } => {
            execute_unstake(deps, env, addr, hooked_denoms)
        }
    }
}

pub(crate) fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooked_denoms = get_hook_caller_registered_denoms(deps.as_ref(), info)?;

    // Get the addresses of members whose voting power has changed.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        for denom in hooked_denoms.clone() {
            update_rewards(&mut deps, &env, &addr, denom)?;
        }
    }

    Ok(Response::new().add_attribute("action", "membership_changed"))
}

pub(crate) fn execute_nft_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: NftStakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooked_denoms = get_hook_caller_registered_denoms(deps.as_ref(), info)?;

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr, hooked_denoms),
        NftStakeChangedHookMsg::Unstake { addr, .. } => {
            execute_unstake(deps, env, addr, hooked_denoms)
        }
    }
}

pub(crate) fn execute_stake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooked_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    // update rewards for every denom that the hook caller is registered for
    for denom in hooked_denoms {
        update_rewards(&mut deps, &env, &addr, denom)?;
    }
    Ok(Response::new().add_attribute("action", "stake"))
}

pub(crate) fn execute_unstake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooked_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    // update rewards for every denom that the hook caller is registered for
    for denom in hooked_denoms {
        update_rewards(&mut deps, &env, &addr, denom)?;
    }
    Ok(Response::new().add_attribute("action", "unstake"))
}
