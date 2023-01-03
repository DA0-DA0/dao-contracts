use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
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
        self.assert_schedule_vests_amount()?;

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

    /// Asserts the vesting schedule decreases to 0 eventually, 2and is never more than the
    /// amount being sent. If it doesn't match these conditions, returns an error.
    pub fn assert_schedule_vests_amount(&self) -> Result<(), ContractError> {
        self.vesting_schedule.validate_monotonic_decreasing()?;
        let (low, high) = self.vesting_schedule.range();
        if low != 0 {
            Err(ContractError::NeverFullyVested)
        } else if high > self.amount.u128() {
            Err(ContractError::VestsMoreThanSent)
        } else {
            Ok(())
        }
    }
}

#[cw_serde]
pub enum VestingPaymentStatus {
    Active,
    Canceled,
    FullyVested,
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
    /// Title of the payroll item, for example for a bug bounty "Fix issue in contract.rs"
    pub title: Option<String>,
    /// Description of the payroll item, a more in depth description of how to meet the payroll conditions
    pub description: Option<String>,
    /// The status of the vesting payment
    pub status: VestingPaymentStatus,
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
            status: VestingPaymentStatus::Active,
            claimed_amount: Uint128::zero(),
            recipient: checked_vesting_params.recipient,
            amount: checked_vesting_params.amount,
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
    // // TODO some tests for piecewise curves
    // use wynd_utils::PiecewiseLinear;

    use super::*;

    #[test]
    fn test_catches_vests_more_than_sent() {
        let amount = Uint128::new(10);
        let start = mock_env().block.time.seconds();
        let end = start + 10_000;
        let vesting_schedule = Curve::saturating_linear((start, 69), (end, 0));
        let params = UncheckedVestingParams {
            recipient: "test".to_string(),
            amount,
            denom: UncheckedDenom::Cw20("addr".to_string()),
            vesting_schedule,
            title: None,
            description: None,
        };
        let err = params.assert_schedule_vests_amount().unwrap_err();
        assert_eq!(err, ContractError::VestsMoreThanSent);
    }

    #[test]
    fn test_catches_never_fully_vesting() {
        let amount = Uint128::new(11223344);
        let start = mock_env().block.time.seconds();
        let end = start + 10_000;
        let vesting_schedule = Curve::saturating_linear((start, amount.into()), (end, 1));
        let params = UncheckedVestingParams {
            recipient: "test".to_string(),
            amount,
            denom: UncheckedDenom::Cw20("addr".to_string()),
            vesting_schedule,
            title: None,
            description: None,
        };
        let err = params.assert_schedule_vests_amount().unwrap_err();
        assert_eq!(err, ContractError::NeverFullyVested);
    }

    #[test]
    fn test_catches_non_decreasing_curve() {
        let amount = Uint128::new(11223344);
        let start = mock_env().block.time.seconds();
        let end = start + 10_000;
        let vesting_schedule = Curve::saturating_linear((start, 0), (end, amount.into()));
        let params = UncheckedVestingParams {
            recipient: "test".to_string(),
            amount,
            denom: UncheckedDenom::Cw20("addr".to_string()),
            vesting_schedule,
            title: None,
            description: None,
        };
        let err = params.assert_schedule_vests_amount().unwrap_err();
        assert_eq!(
            err,
            ContractError::Curve(wynd_utils::CurveError::MonotonicIncreasing)
        );
    }

    // // TODO limit complexity for piecewise linear curves
    // #[test]
    // fn test_complex_vessting_schedule() {
    //     let amount = Uint128::new(11223344);
    //     let start = mock_env().block.time.seconds();
    //     let complexity = 100;
    //     let steps: Vec<_> = (0..complexity)
    //         .map(|x| (start + x, amount - Uint128::from(x)))
    //         .chain(std::iter::once((start + complexity, Uint128::new(0)))) // fully vest
    //         .collect();
    //     let schedule = Curve::PiecewiseLinear(PiecewiseLinear {
    //         steps: steps.clone(),
    //     });
    // }
}
