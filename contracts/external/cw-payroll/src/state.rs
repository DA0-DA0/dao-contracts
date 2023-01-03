use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Response, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use crate::{msg::VestingParams, ContractError};
use cw_denom::CheckedDenom;

use wynd_utils::Curve;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VestingPayment {
    pub recipient: Addr,
    /// Vesting amount in Native and Cw20 tokens
    pub amount: Uint128,
    /// Amount claimed so far
    pub claimed_amount: Uint128,
    /// Vesting schedule
    pub vesting_schedule: Curve,
    /// The denom of a token (cw20 or native)
    pub denom: CheckedDenom,
    // /// The start time in seconds
    // pub start_time: u64,
    // /// The end time in seconds
    // pub end_time: u64,
    // pub paused_time: Option<u64>,
    // pub paused_duration: Option<u64>,
    pub paused: bool,
    /// Title of the payroll item, for example for a bug bounty "Fix issue in contract.rs"
    pub title: Option<String>,
    /// Description of the payroll item, a more in depth description of how to meet the payroll conditions
    pub description: Option<String>,
}

impl VestingPayment {
    /// Asserts the vesting schedule decreases to 0 eventually, and is never more than the
    /// amount being sent. If it doesn't match these conditions, returns an error.
    pub fn assert_schedule_vests_amount(&self, amount: Uint128) -> Result<(), ContractError> {
        self.vesting_schedule.validate_monotonic_decreasing()?;
        let (low, high) = self.vesting_schedule.range();
        if low != 0 {
            Err(ContractError::NeverFullyVested)
        } else if high > amount.u128() {
            Err(ContractError::VestsMoreThanSent)
        } else {
            Ok(())
        }
    }

    //// Contvert from VestingParams
    // pub fn from(&self, vesting_params: VestingParams) -> Result<(), ContractError> {
    // }
}

pub const VESTING_PAYMENT_SEQ: Item<u64> = Item::new("vesting_payment_seq");
pub const VESTING_PAYMENTS: Map<u64, VestingPayment> = Map::new("vesting_payments");

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_env;
    use wynd_utils::PiecewiseLinear;

    use super::*;

    #[test]
    fn test_linear_vesting_schedule() {
        let amount = Uint128::new(11223344);
        let start = mock_env().block.time.seconds();
        let end = start + 10_000;
        let schedule = Curve::saturating_linear((start, 10000000), (end, 0));
        // println!("Linear {:?}", schedule);
    }

    #[test]
    fn test_complex_vessting_schedule() {
        let amount = Uint128::new(11223344);
        // curve is not fully vested yet and complexity is too high
        let start = mock_env().block.time.seconds();
        let complexity = 100;
        let steps: Vec<_> = (0..complexity)
            .map(|x| (start + x, amount - Uint128::from(x)))
            .chain(std::iter::once((start + complexity, Uint128::new(0)))) // fully vest
            .collect();
        let schedule = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: steps.clone(),
        });
        // println!("PieceWiseLinear {:?}", schedule);
    }
}
