use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_denom::CheckedDenom;
use wynd_utils::CurveError;

use crate::vesting::{VestingError, VestingPayment, VestingSchedule};

#[test]
fn test_vesting_validation() {
    // Can not create vesting payment which vests zero tokens.
    assert_eq!(
        VestingPayment::new(
            Uint128::zero(),
            VestingSchedule::SaturatingLinear,
            Timestamp::from_seconds(0),
            100,
            CheckedDenom::Native("native".to_string()),
            Addr::unchecked("ekez")
        ),
        Err(VestingError::NothingToVest {})
    );

    assert_eq!(
        VestingPayment::new(
            Uint128::new(3),
            VestingSchedule::PeacewiseLinear(vec![
                (0, Uint128::zero()),
                (1, Uint128::one()),
                (2, Uint128::zero()), // non-monotonic-increasing
                (3, Uint128::new(3))
            ]),
            Timestamp::from_seconds(0),
            100,
            CheckedDenom::Native("native".to_string()),
            Addr::unchecked("ekez")
        ),
        Err(VestingError::Curve(CurveError::PointsOutOfOrder))
    );

    // Doesn't reach total.
    assert_eq!(
        VestingPayment::new(
            Uint128::new(3),
            VestingSchedule::PeacewiseLinear(vec![
                (1, Uint128::zero()),
                (2, Uint128::one()),
                (5, Uint128::new(2))
            ]),
            Timestamp::from_seconds(0),
            100,
            CheckedDenom::Native("native".to_string()),
            Addr::unchecked("ekez")
        ),
        Err(VestingError::VestRange {
            min: Uint128::zero(),
            max: Uint128::new(2)
        })
    );
}
