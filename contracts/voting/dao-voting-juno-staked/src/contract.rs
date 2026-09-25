#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use dao_hooks::stake::{stake_hook_msgs, unstake_hook_msgs};
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::bindings::{JunoQuerier, JunoQuery};
use crate::error::ContractError;
use crate::msg::{
    DelegationEvent, ExecuteMsg, GetHooksResponse, InstantiateMsg, QueryMsg, SudoMsg,
};
use crate::state::{DAO, HOOKS};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-juno-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<JunoQuery>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DAO.save(deps.storage, &info.sender)?;

    // Registration with x/cw-hooks happens out-of-band post-instantiate
    // (CLI `junod tx cw-hooks register-staking <contract>` or a separate
    // governance/DAO proposal). Building the `MsgRegisterStaking` proto
    // body in-contract would mean pulling in a prost codegen dependency
    // just for one message shape — deferred until a concrete need for
    // single-tx setup appears. See `auto_register_staking_hooks` in
    // `InstantiateMsg`: currently must be `None` / `Some(false)`.
    if msg.auto_register_staking_hooks.unwrap_or(false) {
        return Err(ContractError::AutoRegisterNotYetSupported {});
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("dao", info.sender.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<JunoQuery>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, info, addr),
    }
}

fn execute_add_hook(
    deps: DepsMut<JunoQuery>,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    only_dao(deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, addr.clone())?;
    Ok(Response::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr.to_string()))
}

fn execute_remove_hook(
    deps: DepsMut<JunoQuery>,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    only_dao(deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, addr.clone())?;
    Ok(Response::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr.to_string()))
}

fn only_dao(deps: Deps<JunoQuery>, sender: &Addr) -> Result<(), ContractError> {
    let dao = DAO.load(deps.storage)?;
    if *sender != dao {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut<JunoQuery>, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        // Delegation events that actually move a delegator's bonded
        // power. The chain's x/voting-snapshot fires first in the
        // staking-hook chain (see app/keepers/keepers.go ordering), so
        // by the time our sudo runs the snapshot at `env.block.height`
        // is already written and queryable.
        SudoMsg::AfterDelegationModified {
            after_delegation_modified: ev,
        } => sudo_stake_delta(deps, env, ev, StakeDirection::AfterEvent),
        SudoMsg::BeforeDelegationRemoved {
            before_delegation_removed: ev,
        } => sudo_stake_delta(deps, env, ev, StakeDirection::BeforeRemoval),
        // BeforeDelegationCreated / BeforeDelegationSharesModified fire
        // *before* the snapshot is written, so the new power isn't
        // queryable yet. The after-event variants above carry the same
        // delegator so we get a clean post-event read there; ignoring
        // the pre-event variants avoids double-firing or stale reads.
        SudoMsg::BeforeDelegationCreated { .. }
        | SudoMsg::BeforeDelegationSharesModified { .. } => Ok(Response::new()),
        SudoMsg::BeforeValidatorSlashed {
            before_validator_slashed: _,
        } => {
            // Slashing redistributes power across many delegators at
            // once. We don't enumerate them here — the chain's
            // x/voting-snapshot has lazy per-delegator decay (see
            // memory/v30-upgrade-plan.md B1+B2+B4) so callers that
            // re-query power at the slash height get correct values.
            // Subscribers that need a notification can register with
            // x/cw-hooks directly.
            Ok(Response::new())
        }
        // Validator lifecycle events don't move any single delegator's
        // bonded power on their own; ignoring them keeps cw-hooks
        // happy without firing meaningless stake hooks.
        SudoMsg::AfterValidatorCreated { .. }
        | SudoMsg::AfterValidatorRemoved { .. }
        | SudoMsg::BeforeValidatorModified { .. }
        | SudoMsg::AfterValidatorModified { .. }
        | SudoMsg::AfterValidatorBonded { .. }
        | SudoMsg::AfterValidatorBeginUnbonding { .. } => Ok(Response::new()),
    }
}

enum StakeDirection {
    /// The event fired after a delegation was modified — the chain has
    /// the new total bonded for the delegator in the current-height
    /// snapshot.
    AfterEvent,
    /// The event fired before a delegation was wiped — at sudo time the
    /// chain still has the old non-zero entry in storage. We emit
    /// Unstake for the full pre-removal amount and clear our internal
    /// view to zero.
    BeforeRemoval,
}

fn sudo_stake_delta(
    deps: DepsMut<JunoQuery>,
    env: Env,
    ev: DelegationEvent,
    direction: StakeDirection,
) -> Result<Response, ContractError> {
    let delegator = deps.api.addr_validate(&ev.delegator_address)?;

    let new_power = match direction {
        StakeDirection::AfterEvent => deps
            .querier
            .voting_power_at(delegator.to_string(), env.block.height)?,
        // For a removal that hasn't yet committed, the snapshot at
        // current height still shows the pre-removal stake. Report
        // zero — this delegation is about to be gone, and any
        // subscriber tracking "what's the delegator's voting power
        // now" wants the post-removal value.
        StakeDirection::BeforeRemoval => Uint128::zero(),
    };

    let prev_power = previous_power(deps.as_ref(), delegator.to_string(), env.block.height)?;

    let hooks = HOOKS;
    let submsgs = if new_power >= prev_power {
        let amount = new_power.checked_sub(prev_power).unwrap_or_default();
        if amount.is_zero() {
            return Ok(Response::new().add_attribute("action", "stake_unchanged"));
        }
        stake_hook_msgs(hooks, deps.storage, delegator.clone(), amount)?
    } else {
        let amount = prev_power.checked_sub(new_power).unwrap_or_default();
        unstake_hook_msgs(hooks, deps.storage, delegator.clone(), amount)?
    };

    Ok(Response::new()
        .add_submessages(submsgs)
        .add_attribute("action", "stake_change_hook")
        .add_attribute("delegator", delegator.to_string())
        .add_attribute("new_power", new_power.to_string())
        .add_attribute("prev_power", prev_power.to_string()))
}

/// Returns the snapshot recorded immediately before `current_height`. The
/// chain's at-or-before semantics give us this for free: querying at
/// `current_height - 1` returns whatever the previous event wrote.
/// Saturates at 0 for genesis-block edge cases.
fn previous_power(
    deps: Deps<JunoQuery>,
    address: String,
    current_height: u64,
) -> StdResult<Uint128> {
    if current_height == 0 {
        return Ok(Uint128::zero());
    }
    deps.querier.voting_power_at(address, current_height - 1)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut<JunoQuery>, _env: Env, _reply: Reply) -> Result<Response, ContractError> {
    // Reserved for future auto-unregistration on hook failure. The
    // current dao_hooks call sites use plain SubMsg::new (no reply
    // requested), so this entry-point is unreachable in practice.
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<JunoQuery>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_json_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_json_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Dao {} => to_json_binary(&DAO.load(deps.storage)?),
        QueryMsg::Info {} => to_json_binary(&InfoResponse {
            info: cw2::get_contract_version(deps.storage)?,
        }),
        QueryMsg::GetHooks {} => to_json_binary(&GetHooksResponse {
            hooks: HOOKS.query_hooks(deps)?.hooks,
        }),
    }
}

fn query_voting_power_at_height(
    deps: Deps<JunoQuery>,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = deps.querier.voting_power_at(address, height)?;
    Ok(VotingPowerAtHeightResponse { power, height })
}

fn query_total_power_at_height(
    deps: Deps<JunoQuery>,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = deps.querier.total_voting_power_at(height)?;
    Ok(TotalPowerAtHeightResponse { power, height })
}
