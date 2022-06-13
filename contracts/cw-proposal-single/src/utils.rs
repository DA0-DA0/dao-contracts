use cosmwasm_std::{Addr, Deps, StdResult, Uint128};

use cw_core_interface::voting;
use cw_utils::Duration;

use crate::ContractError;

pub fn get_voting_power(
    deps: Deps,
    address: Addr,
    dao: Addr,
    height: Option<u64>,
) -> StdResult<Uint128> {
    let response: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        dao,
        &voting::Query::VotingPowerAtHeight {
            address: address.to_string(),
            height,
        },
    )?;
    Ok(response.power)
}

pub fn get_total_power(deps: Deps, dao: Addr, height: Option<u64>) -> StdResult<Uint128> {
    let response: voting::TotalPowerAtHeightResponse = deps
        .querier
        .query_wasm_smart(dao, &voting::Query::TotalPowerAtHeight { height })?;
    Ok(response.power)
}

/// Validates that the min voting period is less than the max voting
/// period. Passes arguments through the function.
pub fn validate_voting_period(
    min: Option<Duration>,
    max: Duration,
) -> Result<(Option<Duration>, Duration), ContractError> {
    let min = min
        .map(|min| {
            let valid = match (min, max) {
                (Duration::Time(min), Duration::Time(max)) => min <= max,
                (Duration::Height(min), Duration::Height(max)) => min <= max,
                _ => return Err(ContractError::DurationUnitsConflict {}),
            };
            if valid {
                Ok(min)
            } else {
                Err(ContractError::InvalidMinVotingPeriod {})
            }
        })
        .transpose()?;

    Ok((min, max))
}
