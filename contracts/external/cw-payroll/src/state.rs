use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Uint128};
use cw_storage_plus::{Item, Map};

use crate::ContractError;
use cw_denom::{CheckedDenom, UncheckedDenom};

use wynd_utils::Curve;

#[cw_serde]
pub struct UncheckedVestingParams {
    pub recipient: String,
    pub amount: Uint128,
    pub denom: UncheckedDenom,
    pub vesting_schedule: Curve,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[cw_serde]
pub struct CheckedVestingParams {
    pub recipient: Addr,
    pub amount: Uint128,
    pub denom: CheckedDenom,
    pub vesting_schedule: Curve,
    pub title: Option<String>,
    pub description: Option<String>,
}

impl UncheckedVestingParams {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedVestingParams, ContractError> {
        // Check vesting schedule
        self.assert_schedule_vests_amount(self.amount)?;

        // Check valid recipient address
        let recipient = deps.api.addr_validate(&self.recipient)?;

        // Check denom
        let checked_denom = match self.denom {
            UncheckedDenom::Native(denom) => UncheckedDenom::Native(denom).into_checked(deps)?,
            UncheckedDenom::Cw20(addr) => UncheckedDenom::Cw20(addr).into_checked(deps)?,
        };

        Ok(CheckedVestingParams {
            recipient,
            amount: self.amount,
            denom: checked_denom,
            vesting_schedule: self.vesting_schedule,
            title: self.title,
            description: self.description,
        })
    }

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
}

#[cw_serde]
pub struct VestingPayment {
    /// The ID of the vesting payment
    pub id: u64,
    /// The recipient for the vesting payment
    pub recipient: Addr,
    /// Vesting amount in Native and Cw20 tokens
    pub amount: Uint128,
    /// Amount claimed so far
    pub claimed_amount: Uint128,
    /// Vesting schedule
    pub vesting_schedule: Curve,
    /// The denom of a token (cw20 or native)
    pub denom: CheckedDenom,
    /// Whether the payment is paused
    pub paused: bool,
    /// Title of the payroll item, for example for a bug bounty "Fix issue in contract.rs"
    pub title: Option<String>,
    /// Description of the payroll item, a more in depth description of how to meet the payroll conditions
    pub description: Option<String>,
}

impl VestingPayment {
    /// Create a new VestingPayment from CheckedVestingParams
    pub fn new(
        deps: DepsMut,
        checked_vesting_params: CheckedVestingParams,
    ) -> Result<Self, ContractError> {
        let mut id = VESTING_PAYMENT_SEQ.load(deps.storage)?;
        id += 1;

        let vesting_payment = Self {
            id,
            paused: false,
            recipient: checked_vesting_params.recipient,
            amount: checked_vesting_params.amount,
            claimed_amount: Uint128::zero(),
            denom: checked_vesting_params.denom,
            vesting_schedule: checked_vesting_params.vesting_schedule,
            title: checked_vesting_params.title,
            description: checked_vesting_params.description,
        };

        VESTING_PAYMENT_SEQ.save(deps.storage, &id)?;
        VESTING_PAYMENTS.save(deps.storage, id, &vesting_payment)?;

        Ok(vesting_payment)
    }
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
