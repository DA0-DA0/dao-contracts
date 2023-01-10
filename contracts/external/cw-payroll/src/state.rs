use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, OverflowError, Uint128};
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

        // If title is included, validate title length (max 280 characters)
        if let Some(ref title) = self.title {
            if title.len() > 280 || title.is_empty() {
                return Err(ContractError::InvalidTitle);
            }
        }

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
    CanceledAndUnbonding,
    FullyVested,
    Unfunded,
}

#[cw_serde]
pub struct VestingPayment {
    /// The recipient for the vesting payment
    pub recipient: Addr,
    /// Vesting amount in Native and Cw20 tokens
    pub amount: Uint128,
    /// Amount claimed so far
    pub claimed_amount: Uint128,
    /// Canceled at time in seconds, only set if contract is canceled
    pub canceled_at_time: Option<u64>,
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
    /// The amount of the vesting payment that has been staked
    pub staked_amount: Uint128,
    /// Info about staked vesting payment rewards
    pub rewards: VestingPaymentRewards,
}

impl VestingPayment {
    /// Create a new VestingPayment from CheckedVestingParams
    pub fn new(checked_vesting_params: CheckedVestingParams) -> Result<Self, ContractError> {
        let vesting_payment = Self {
            status: VestingPaymentStatus::Active,
            claimed_amount: Uint128::zero(),
            staked_amount: Uint128::zero(),
            canceled_at_time: None,
            recipient: checked_vesting_params.recipient,
            amount: checked_vesting_params.amount,
            denom: checked_vesting_params.denom,
            vesting_schedule: checked_vesting_params.vesting_schedule,
            title: checked_vesting_params.title,
            description: checked_vesting_params.description,
            rewards: VestingPaymentRewards {
                pending: Decimal::zero(),
                paid_rewards_per_token: Decimal::zero(),
            },
        };

        Ok(vesting_payment)
    }

    pub fn get_vested_amount_by_seconds(&self, time: u64) -> Result<Uint128, OverflowError> {
        let vesting_funds = self.vesting_schedule.value(time);
        self.amount.checked_sub(vesting_funds)
    }

    pub fn calc_pending_rewards(
        &mut self,
        new_rewards_per_token: Decimal,
        staked: Uint128,
    ) -> Result<(), ContractError> {
        // No staked amount, so no rewards
        if staked.is_zero() {
            self.rewards.paid_rewards_per_token = new_rewards_per_token;
            return Ok(());
        }

        let rewards_per_token_to_pay = new_rewards_per_token - self.rewards.paid_rewards_per_token;

        // We don't need to update anything, nothing to calculate
        if rewards_per_token_to_pay.is_zero() {
            return Ok(());
        }

        self.rewards.pending +=
            rewards_per_token_to_pay.checked_mul(Decimal::from_atomics(staked, 0)?)?;

        self.rewards.paid_rewards_per_token = new_rewards_per_token;

        Ok(())
    }

    pub fn reset_pending_rewards(&mut self) {
        self.rewards.pending -= self.rewards.pending.floor();
    }

    /// Turn pending decimal to u128 to send tokens
    pub fn get_pending_rewards(&self) -> Result<Uint128, ContractError> {
        // TODO: Find a better way of doing this?
        let decimal_fractional = Uint128::from(
            10_u128
                .checked_pow(self.rewards.pending.decimal_places())
                .unwrap_or(1_000_000_000_000_000_000u128),
        );
        let full_num = self.rewards.pending.floor().atomics();
        let to_send = full_num.checked_div(decimal_fractional)?;
        Ok(to_send)
    }
}

/// A map of vesting payments (ID, VestingPayment)
pub const VESTING_PAYMENT: Item<VestingPayment> = Item::new("vesting_payment");
/// A map of staked vesting claims by validator
pub const STAKED_VESTING_BY_VALIDATOR: Map<&str, Uint128> = Map::new("staked_vesting_by_validator");
/// A map that keeps track of withdrawn rewards for a particular validator
pub const VALIDATORS_REWARDS: Map<&str, ValidatorRewards> = Map::new("validators_rewards");

#[cw_serde]
pub struct ValidatorRewards {
    /// rewards_per_token, total of rewards to be paid per staked token.
    pub rewards_per_token: Decimal,
}

impl Default for ValidatorRewards {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidatorRewards {
    pub fn new() -> Self {
        ValidatorRewards {
            rewards_per_token: Decimal::zero(),
        }
    }

    pub fn calc_rewards(
        &mut self,
        rewards: Uint128,
        total_tokens: Uint128,
    ) -> Result<(), ContractError> {
        let rewards_dec = Decimal::checked_from_ratio(rewards, total_tokens)?;

        self.rewards_per_token = rewards_dec.checked_add(self.rewards_per_token)?;
        Ok(())
    }
}

#[cw_serde]
pub struct VestingPaymentRewards {
    pub pending: Decimal,
    pub paid_rewards_per_token: Decimal,
}

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
