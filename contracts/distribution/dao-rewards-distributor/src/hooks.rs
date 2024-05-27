use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult};
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};

use crate::{contract::update_rewards, state::REGISTERED_HOOKS, ContractError};

pub fn subscribe_denom_to_hook(
    deps: DepsMut,
    hook: Addr,
    denom: String,
) -> Result<(), ContractError> {
    REGISTERED_HOOKS.update(deps.storage, hook, |denoms| -> StdResult<_> {
        let mut denoms = denoms.unwrap_or_default();
        denoms.push(denom.to_string());
        Ok(denoms)
    })?;
    Ok(())
}

/// Ensures hooks that update voting power are only called by a designated
/// hook_caller contract.
/// Returns a list of denoms that the hook caller is registered for.
pub fn check_hook_caller(deps: Deps, info: MessageInfo) -> Result<Vec<String>, ContractError> {
    // only a designated hook_caller contract can call this hook.
    // failing to load the registered denoms for a given hook returns an error.
    REGISTERED_HOOKS
        .load(deps.storage, info.sender.clone())
        .map_err(|_| ContractError::InvalidHookSender {})
}

pub fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooks = check_hook_caller(deps.as_ref(), info)?;

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr, hooks),
        StakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr, hooks),
    }
}

pub fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooks = check_hook_caller(deps.as_ref(), info)?;

    // Get the addresses of members whose voting power has changed.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        update_rewards(&mut deps, &env, &addr, hooks.clone())?;
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
    let hooks = check_hook_caller(deps.as_ref(), info)?;

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr, hooks),
        NftStakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr, hooks),
    }
}

pub fn execute_stake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooks: Vec<String>,
) -> Result<Response, ContractError> {
    update_rewards(&mut deps, &env, &addr, hooks)?;
    Ok(Response::new().add_attribute("action", "stake"))
}

pub fn execute_unstake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooks: Vec<String>,
) -> Result<Response, ContractError> {
    update_rewards(&mut deps, &env, &addr, hooks)?;
    Ok(Response::new().add_attribute("action", "unstake"))
}
