use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Timestamp, Uint128,
};
use cw_denom::CheckedDenom;
use wynd_utils::CurveError;

use crate::{
    error::ContractError,
    vesting::{Payment, Schedule, Status, Vest, VestInit},
};

impl Default for VestInit {
    fn default() -> Self {
        VestInit {
            total: Uint128::new(100_000_000),
            schedule: Schedule::SaturatingLinear,
            start_time: Timestamp::from_seconds(0),
            duration_seconds: 100,
            denom: CheckedDenom::Native("native".to_string()),
            recipient: Addr::unchecked("recv"),
            title: "title".to_string(),
            description: "desc".to_string(),
        }
    }
}

#[test]
fn test_distribute_not_funded() {
    let storage = &mut mock_dependencies().storage;
    let payment = Payment::new("vesting", "staking");

    payment.initialize(storage, VestInit::default()).unwrap();
    payment.set_funded(storage).unwrap();

    payment
        .distribute(storage, &Timestamp::default().plus_seconds(10), None)
        .unwrap();
}

#[test]
fn test_distribute_nothing_to_claim() {
    let storage = &mut mock_dependencies().storage;
    let payment = Payment::new("vesting", "staking");
    payment.initialize(storage, VestInit::default()).unwrap();

    payment.set_funded(storage).unwrap();

    // Can't distribute when there is nothing to claim.
    let err = payment
        .distribute(storage, &Timestamp::default(), None)
        .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidWithdrawal {
            request: Uint128::zero(),
            claimable: Uint128::zero()
        }
    );
}

#[test]
fn test_distribute_half_way() {
    let storage = &mut mock_dependencies().storage;
    let payment = Payment::new("vesting", "staking");
    payment.initialize(storage, VestInit::default()).unwrap();

    payment.set_funded(storage).unwrap();
    // 50% of the way through, max claimable is 1/2 total.
    let err = payment
        .distribute(
            storage,
            &Timestamp::from_seconds(50),
            Some(Uint128::new(50_000_001)),
        )
        .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidWithdrawal {
            request: Uint128::new(50_000_001),
            claimable: Uint128::new(50_000_000)
        }
    );
}

#[test]
fn test_distribute() {
    let storage = &mut mock_dependencies().storage;
    let payment = Payment::new("vesting", "staking");
    payment.initialize(storage, VestInit::default()).unwrap();

    payment.set_funded(storage).unwrap();

    // partially claiming increases claimed
    let msg = payment
        .distribute(storage, &Timestamp::from_seconds(50), Some(Uint128::new(3)))
        .unwrap();

    assert_eq!(
        msg,
        payment
            .get_vest(storage)
            .unwrap()
            .denom
            .get_transfer_to_message(&Addr::unchecked("recv"), Uint128::new(3))
            .unwrap()
    );
    assert_eq!(payment.get_vest(storage).unwrap().claimed, Uint128::new(3));

    payment
        .distribute(
            storage,
            &Timestamp::from_seconds(50),
            Some(Uint128::new(50_000_000 - 3)),
        )
        .unwrap();
}

#[test]
fn test_vesting_validation() {
    // Can not create vesting payment which vests zero tokens.
    let init = VestInit {
        total: Uint128::zero(),
        ..Default::default()
    };
    assert_eq!(Vest::new(init), Err(ContractError::ZeroVest {}));

    let init = VestInit {
        schedule: Schedule::PeacewiseLinear(vec![
            (0, Uint128::zero()),
            (1, Uint128::one()),
            (2, Uint128::zero()), // non-monotonic-increasing
            (3, Uint128::new(3)),
        ]),
        ..Default::default()
    };

    assert_eq!(
        Vest::new(init),
        Err(ContractError::Curve(CurveError::PointsOutOfOrder))
    );

    // Doesn't reach total.
    let init = VestInit {
        schedule: Schedule::PeacewiseLinear(vec![
            (1, Uint128::zero()),
            (2, Uint128::one()),
            (5, Uint128::new(2)),
        ]),
        ..Default::default()
    };

    assert_eq!(
        Vest::new(init),
        Err(ContractError::VestRange {
            min: Uint128::zero(),
            max: Uint128::new(2)
        })
    );
}

// owner and vestee. vestee has vested 50 tokens out of 100. 10 are
// claimed, 15 liquid, and 75 staked. owner then cancels the vest.
//
// the 15 liquid tokens should then all be sent to the vestee as the
// contract prioritises making them whole first. the vestee is now
// owed 25 tokens, and the owner 50.
//
// now the owner triggers an unbonding of 50 tokens. once they unbond,
// the vestee uses those tokens to make themselves whole. the owner
// may withdraw 25 tokens at this point, and later unbond the
// remaining 25 tokens and make themselves whole.
#[test]
fn test_complex_close() {
    let storage = &mut mock_dependencies().storage;
    let mut time = Timestamp::default();

    let init = VestInit {
        total: Uint128::new(100),
        schedule: Schedule::SaturatingLinear,
        start_time: time,
        duration_seconds: 100,
        denom: CheckedDenom::Native("ujuno".to_string()),
        recipient: Addr::unchecked("recv"),
        title: "t".to_string(),
        description: "d".to_string(),
    };
    let payment = Payment::new("vesting", "staking");

    payment.initialize(storage, init).unwrap();
    payment.set_funded(storage).unwrap();

    time = time.plus_seconds(50);

    payment
        .distribute(storage, &time, Some(Uint128::new(10)))
        .unwrap();

    payment.delegate(storage, &time, Uint128::new(75)).unwrap();

    let vest = payment.get_vest(storage).unwrap();
    assert_eq!(vest.claimed, Uint128::new(10));
    assert_eq!(vest.vested(&time), Uint128::new(50));

    let mut env = mock_env();
    env.block.time = time;
    payment
        .cancel(storage, &env, &Addr::unchecked("owner"))
        .unwrap();

    let vest = payment.get_vest(storage).unwrap();
    assert_eq!(
        vest.status,
        Status::Canceled {
            owner_withdrawable: Uint128::new(50)
        }
    );
    assert_eq!(vest.vested(&time) - vest.claimed, Uint128::new(25));

    payment
        .undelegate(storage, &time, Uint128::new(50), 25)
        .unwrap();
    time = time.plus_seconds(25);

    payment.distribute(storage, &time, None).unwrap();
    payment
        .withdraw_canceled(storage, &time, None, &Addr::unchecked("owner"))
        .unwrap();

    let vest = payment.get_vest(storage).unwrap();
    assert_eq!(vest.claimed, Uint128::new(50));
    assert_eq!(vest.total(), Uint128::new(50));
    assert_eq!(
        vest.status,
        Status::Canceled {
            owner_withdrawable: Uint128::new(25)
        }
    );

    payment
        .undelegate(storage, &time, Uint128::new(25), 25)
        .unwrap();
    time = time.plus_seconds(25);
    payment
        .withdraw_canceled(storage, &time, None, &Addr::unchecked("owner"))
        .unwrap();
    let vest = payment.get_vest(storage).unwrap();
    assert_eq!(
        vest.status,
        Status::Canceled {
            owner_withdrawable: Uint128::zero()
        }
    );
}
