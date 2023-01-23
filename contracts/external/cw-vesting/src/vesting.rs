use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, StdResult, Timestamp, Uint128};
use cw_denom::CheckedDenom;
use thiserror::Error;
use wynd_utils::{Curve, CurveError};

#[cw_serde]
pub struct VestingPayment {
    /// vested(t), where t is in seconds since start time.
    vested: Curve,
    start_time: Timestamp,
    denom: CheckedDenom,
    receiver: Addr,

    pub claimed: Uint128,
}

#[cw_serde]
pub enum VestingSchedule {
    SaturatingLinear,
    PeacewiseLinear(Vec<(u64, Uint128)>),
}

#[derive(Error, Debug, PartialEq)]
pub enum VestingError {
    #[error(transparent)]
    Curve(#[from] CurveError),

    #[error("vesting curve values be in [0, total]`. got [{min}, {max}]")]
    VestRange { min: Uint128, max: Uint128 },

    #[error("total amount to vest must be non-zero")]
    NothingToVest {},
}

impl VestingPayment {
    pub fn new(
        total: Uint128,
        schedule: VestingSchedule,
        start_time: Timestamp,
        duration_seconds: u64,
        denom: CheckedDenom,
        receiver: Addr,
    ) -> Result<Self, VestingError> {
        if total.is_zero() {
            Err(VestingError::NothingToVest {})
        } else {
            Ok(Self {
                claimed: Uint128::zero(),
                vested: schedule.into_curve(total, duration_seconds)?,
                start_time,
                denom,
                receiver,
            })
        }
    }

    /// Gets the total number of tokens that will vest as part of this
    /// payment.
    pub fn total(&self) -> Uint128 {
        Uint128::new(self.vested.range().1)
    }

    /// Gets the number of tokens that have vested at `time`.
    pub fn vested(&self, time: Timestamp) -> Uint128 {
        let elapsed = self.start_time.minus_nanos(time.nanos()).seconds();
        self.vested.value(elapsed)
    }

    /// Gets the number of tokens that are withdrawable at `time`.
    pub fn withdrawable(&self, time: Timestamp) -> Uint128 {
        let vested = self.vested(time);
        vested - self.claimed
    }

    /// Vests all withdrawable tokens. Mutates the instance.
    pub fn execute_vest(&mut self, time: Timestamp) -> StdResult<CosmosMsg> {
        let withdrawable = self.withdrawable(time);
        self.claimed += withdrawable;
        self.denom
            .get_transfer_to_message(&self.receiver, withdrawable)
    }
}

impl VestingSchedule {
    /// The vesting schedule tracks vested(t), so for a curve to be
    /// valid:
    ///
    /// 1. it must start at 0,
    /// 2. it must end at total,
    /// 3. it must never decrease.
    ///
    /// A schedule is valid if `total` is zero: nothing will ever be
    /// paid out. Consumers should consider validating that `total` is
    /// non-zero.
    pub fn into_curve(self, total: Uint128, duration_seconds: u64) -> Result<Curve, VestingError> {
        let c = match self {
            VestingSchedule::SaturatingLinear => {
                Curve::saturating_linear((0, 0), (duration_seconds, total.u128()))
            }
            VestingSchedule::PeacewiseLinear(steps) => {
                Curve::PiecewiseLinear(wynd_utils::PiecewiseLinear { steps })
            }
        };
        c.validate_monotonic_increasing()?;
        let range = c.range();
        if range != (0, total.u128()) {
            return Err(VestingError::VestRange {
                min: Uint128::new(range.0),
                max: Uint128::new(range.1),
            });
        }
        Ok(c)
    }
}
