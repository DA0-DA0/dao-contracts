use cosmwasm_std::{Decimal, Deps, StdResult, Timestamp, Uint128};

use crate::{
    abc::{CommonsPhase, CommonsPhaseConfig, CurveType, VestingSchedule},
    msg::{HatcherAllowlistEntryMsg, QuoteResponse},
    state::{CurveState, HatcherAllowlistConfig, HatcherAllowlistEntry, HatcherState},
    ContractError,
};

/// Calculate the buy quote for a payment
pub fn calculate_buy_quote(
    payment: Uint128,
    curve_type: &CurveType,
    curve_state: &CurveState,
    phase: &CommonsPhase,
    phase_config: &CommonsPhaseConfig,
) -> Result<QuoteResponse, ContractError> {
    // Generate the bonding curve
    let curve_fn = curve_type.to_curve_fn();
    let curve = curve_fn(curve_state.decimals);

    // Calculate the reserved and funded amounts based on the Commons phase
    let (reserved, funded) = match phase {
        CommonsPhase::Hatch => calculate_reserved_and_funded(payment, phase_config.hatch.entry_fee),
        CommonsPhase::Open => calculate_reserved_and_funded(payment, phase_config.open.entry_fee),
        CommonsPhase::Closed => Err(ContractError::CommonsClosed {}),
    }?;

    // Update the reserve and calculate the new supply from the new reserve
    let new_reserve = curve_state.reserve.checked_add(reserved)?;
    let new_supply = curve.supply(new_reserve);

    // Calculate the difference between the new and old supply to get the minted tokens
    let minted = new_supply.checked_sub(curve_state.supply)?;

    Ok(QuoteResponse {
        new_reserve,
        funded,
        amount: minted,
        new_supply,
    })
}

/// Calculate the sell quote for a payment
pub fn calculate_sell_quote(
    payment: Uint128,
    curve_type: &CurveType,
    curve_state: &CurveState,
    phase: &CommonsPhase,
    phase_config: &CommonsPhaseConfig,
) -> Result<QuoteResponse, ContractError> {
    // Generate the bonding curve
    let curve_fn = curve_type.to_curve_fn();
    let curve = curve_fn(curve_state.decimals);

    // Reduce the supply by the amount being burned
    let new_supply = curve_state.supply.checked_sub(payment)?;

    // Determine the exit fee based on the current Commons phase
    let exit_fee = match &phase {
        CommonsPhase::Hatch => Err(ContractError::CommonsHatch {}),
        CommonsPhase::Open => Ok(phase_config.open.exit_fee),
        CommonsPhase::Closed => Ok(Decimal::zero()),
    }?;

    // Calculate the new reserve based on the new supply
    let new_reserve = curve.reserve(new_supply);

    // Calculate how many reserve tokens to release based on the amount being burned
    let released_reserve = curve_state.reserve.checked_sub(new_reserve)?;

    // Calculate the reserved and funded amounts based on the exit fee
    let (reserved, funded) = calculate_reserved_and_funded(released_reserve, exit_fee)?;

    Ok(QuoteResponse {
        new_reserve,
        funded,
        amount: reserved,
        new_supply,
    })
}

/// Return the reserved and funded amounts based on the payment and the allocation ratio
pub(crate) fn calculate_reserved_and_funded(
    payment: Uint128,
    allocation_ratio: Decimal,
) -> Result<(Uint128, Uint128), ContractError> {
    if allocation_ratio.is_zero() {
        return Ok((payment, Uint128::zero()));
    }

    let funded = payment.checked_mul_floor(allocation_ratio)?;
    let reserved = payment - funded; // Since allocation_ratio is < 1, this subtraction is safe

    Ok((reserved, funded))
}

impl HatcherAllowlistEntryMsg {
    pub fn into_entry(&self, deps: Deps, height: u64) -> StdResult<HatcherAllowlistEntry> {
        Ok(HatcherAllowlistEntry {
            addr: deps.api.addr_validate(&self.addr)?,
            config: HatcherAllowlistConfig {
                config_type: self.config.config_type,
                contribution_limits_override: self.config.contribution_limits_override,
                config_height: height,
            },
        })
    }
}

/// Compute the unlocked portion of a hatcher's `minted` balance at time `now`.
///
/// - Returns `state.minted` if vesting hasn't started (still in Hatch),
///   under the assumption that the contract will not allow sells while in
///   Hatch (enforced separately in `calculate_sell_quote`).
/// - Returns `state.minted` for `VestingSchedule::None`.
/// - For `Cliff`, returns 0 until duration has elapsed, then `state.minted`.
/// - For `Linear`, returns `state.minted * elapsed / duration` saturating at
///   `state.minted` once duration has elapsed. Zero-duration treated as
///   immediately fully vested.
pub fn vested_amount(
    state: &HatcherState,
    schedule: &VestingSchedule,
    now: Timestamp,
) -> Uint128 {
    let started = match state.vesting_started_at {
        Some(t) => t,
        // Still in Hatch — sells should be blocked at the phase layer; if
        // they reach here, treat as fully vested so the math is well-defined.
        None => return state.minted,
    };

    if now < started {
        return Uint128::zero();
    }
    let elapsed = now.seconds() - started.seconds();

    match schedule {
        VestingSchedule::None => state.minted,
        VestingSchedule::Cliff { duration_seconds } => {
            if *duration_seconds == 0 || elapsed >= *duration_seconds {
                state.minted
            } else {
                Uint128::zero()
            }
        }
        VestingSchedule::Linear { duration_seconds } => {
            if *duration_seconds == 0 || elapsed >= *duration_seconds {
                state.minted
            } else {
                state
                    .minted
                    .multiply_ratio(Uint128::from(elapsed), Uint128::from(*duration_seconds))
            }
        }
    }
}
