use cosmwasm_std::{from_json, testing::mock_dependencies, Timestamp, Uint128};

use crate::{StakeTracker, StakeTrackerQuery};

#[test]
fn test_stake_tracking() {
    let storage = &mut mock_dependencies().storage;

    let st = StakeTracker::new("s", "v", "c");
    let mut time = Timestamp::from_seconds(0);
    let unbonding_duration_seconds = 100;

    // cardinality, total, and validator_staked start at 0.
    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 0);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::zero());
    assert_eq!(
        st.validator_staked(storage, time, "v1".to_string())
            .unwrap(),
        Uint128::zero()
    );

    // delegating increases validator cardinality, validator_staked, and total.
    st.on_delegate(storage, time, "v1".to_string(), Uint128::new(10))
        .unwrap();

    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 1);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::new(10));
    assert_eq!(
        st.validator_staked(storage, time, "v1".to_string())
            .unwrap(),
        Uint128::new(10)
    );
    // delegating to one validator does not change the status of other validators.
    assert_eq!(
        st.validator_staked(storage, time, "v2".to_string())
            .unwrap(),
        Uint128::zero()
    );

    // delegate to another validator, and undelegate from the first
    // one. the undelegation should not change cardinality or staked
    // values until the unbonding duration has passed.
    st.on_delegate(storage, time, "v2".to_string(), Uint128::new(10))
        .unwrap();
    st.on_undelegate(
        storage,
        time,
        "v1".to_string(),
        Uint128::new(10),
        unbonding_duration_seconds,
    )
    .unwrap();

    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 2);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::new(20));
    assert_eq!(
        st.validator_staked(storage, time, "v1".to_string())
            .unwrap(),
        Uint128::new(10)
    );
    assert_eq!(
        st.validator_staked(storage, time, "v2".to_string())
            .unwrap(),
        Uint128::new(10)
    );

    // after unbonding duration passes, undelegation changes should be
    // visible.
    time = time.plus_seconds(unbonding_duration_seconds);

    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 1);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::new(10));
    assert_eq!(
        st.validator_staked(storage, time, "v1".to_string())
            .unwrap(),
        Uint128::zero()
    );
    assert_eq!(
        st.validator_staked(storage, time, "v2".to_string())
            .unwrap(),
        Uint128::new(10)
    );
}

#[test]
#[should_panic(expected = "attempt to subtract with overflow")]
fn test_undelegation_before_delegation_panics() {
    let storage = &mut mock_dependencies().storage;

    let st = StakeTracker::new("s", "v", "c");

    st.on_delegate(
        storage,
        Timestamp::default(),
        "v2".to_string(),
        Uint128::new(10),
    )
    .unwrap();

    // there are 10 staked tokens total, but they are not staked to
    // this validator so removing them should cause an error.
    st.on_undelegate(
        storage,
        Timestamp::default(),
        "v1".to_string(),
        Uint128::new(10),
        10,
    )
    .unwrap();
}

#[test]
fn test_bonded_slash() {
    let storage = &mut mock_dependencies().storage;
    let st = StakeTracker::new("s", "v", "c");

    st.on_delegate(
        storage,
        Timestamp::from_seconds(10),
        "v1".to_string(),
        Uint128::new(10),
    )
    .unwrap();

    // undelegate half of tokens at t=10.
    st.on_undelegate(
        storage,
        Timestamp::from_seconds(10),
        "v1".to_string(),
        Uint128::new(5),
        5,
    )
    .unwrap();

    // slash the rest at t=12.
    st.on_bonded_slash(
        storage,
        Timestamp::from_seconds(12),
        "v1".to_string(),
        Uint128::new(5),
    )
    .unwrap();

    // at t=13 tokens are still "staked" as this tracks `bonded +
    // unbonding`.
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(13))
            .unwrap(),
        1
    );
    // at t=15 the unbonding has completed and there are no tokens
    // staked. `on_bonded_slash` ought to have updated the
    // cardinality.
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(15))
            .unwrap(),
        0
    );

    // at time t=10, there are five bonded tokens and five unbonding
    // tokens so 10 total staked.
    let staked = st
        .validator_staked(storage, Timestamp::from_seconds(10), "v1".to_string())
        .unwrap();
    assert_eq!(staked, Uint128::new(10));

    // at time t=12 all of the bonded tokens have been slashed, but
    // the unbonding ones are still unbonding.
    let staked = st
        .validator_staked(storage, Timestamp::from_seconds(12), "v1".to_string())
        .unwrap();
    assert_eq!(staked, Uint128::new(5));

    // at time t=15 all of the unbonding has completed and there are
    // no staked tokens.
    let staked = st
        .validator_staked(storage, Timestamp::from_seconds(15), "v1".to_string())
        .unwrap();
    assert_eq!(staked, Uint128::zero());
}

/// t=0 -> bond 10 tokens
/// t=1 -> five tokens slashed, not registered
/// t=2 -> unbond five tokens w/ five second unbonding period
/// t=7 -> cardinality=0 w/ slash considered
/// t=8 -> bond five tokens
/// t=9 -> unbond five tokenw w/ five second unbonding period
///
/// t=9 -> register slash at time t=1
/// t=9 -> cardinality history should now reflect reality.
#[test]
fn test_bonded_slash_updates_cardinality_history() {
    let storage = &mut mock_dependencies().storage;
    let st = StakeTracker::new("s", "v", "c");

    st.on_delegate(
        storage,
        Timestamp::from_seconds(0),
        "v1".to_string(),
        Uint128::new(10),
    )
    .unwrap();
    // t=1 slash of five tokens occurs.
    st.on_undelegate(
        storage,
        Timestamp::from_seconds(2),
        "v1".to_string(),
        Uint128::new(5),
        5,
    )
    .unwrap();

    st.on_delegate(
        storage,
        Timestamp::from_seconds(8),
        "v1".to_string(),
        Uint128::new(5),
    )
    .unwrap();

    // t=7, cardinality=0. but slash not registered so system thinks
    // the cardinality is 1.
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(7))
            .unwrap(),
        1
    );

    // register the slash
    st.on_bonded_slash(
        storage,
        Timestamp::from_seconds(1),
        "v1".to_string(),
        Uint128::new(5),
    )
    .unwrap();

    // t=0, cardinality=1
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(0))
            .unwrap(),
        1
    );
    // t=1, cardinality=1
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(1))
            .unwrap(),
        1
    );

    // t=7, cardinality=0. 5 slashed, 5 unbonded.
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(7))
            .unwrap(),
        0
    );
    // t=8, cardinality=1. 5 bonded.
    assert_eq!(
        st.validator_cardinality(storage, Timestamp::from_seconds(8))
            .unwrap(),
        1
    );
}

/// @t=0, staked to two validators
/// unbonding_duration = 5
///
/// @t=1, unbond from validator 1
/// @t=2, slash of all unbonding tokens for validator 1, cardinality reduced
/// @t=3, unbond from validator 2
/// @t=4, t=2 slash registered
#[test]
fn test_unbonding_slash() {
    let storage = &mut mock_dependencies().storage;
    let st = StakeTracker::new("s", "v", "c");

    let delegation = Uint128::new(10);
    let unbonding_duration = 5;

    // @t=0, staked to two validators
    st.on_delegate(
        storage,
        Timestamp::from_seconds(0),
        "v1".to_string(),
        delegation,
    )
    .unwrap();
    st.on_delegate(
        storage,
        Timestamp::from_seconds(0),
        "v2".to_string(),
        delegation,
    )
    .unwrap();

    // @t=1, unbond from validator 1
    st.on_undelegate(
        storage,
        Timestamp::from_seconds(1),
        "v1".to_string(),
        delegation,
        unbonding_duration,
    )
    .unwrap();

    // @t=3, unbond from validator 2
    st.on_undelegate(
        storage,
        Timestamp::from_seconds(3),
        "v2".to_string(),
        delegation,
        unbonding_duration,
    )
    .unwrap();

    // check that values @t=2 are correct w/o slash registered.
    let total = st
        .total_staked(storage, Timestamp::from_seconds(2))
        .unwrap();
    let cardinality = st
        .validator_cardinality(storage, Timestamp::from_seconds(2))
        .unwrap();
    let v1 = st
        .validator_staked(storage, Timestamp::from_seconds(2), "v1".to_string())
        .unwrap();
    let v2 = st
        .validator_staked(storage, Timestamp::from_seconds(2), "v2".to_string())
        .unwrap();

    assert_eq!(total, delegation + delegation);
    assert_eq!(cardinality, 2);
    assert_eq!(v1, delegation);
    assert_eq!(v2, delegation);

    // check that the cardinality reduces after v1's unbond @t=1.
    let cardinality_after_v1_unbond = st
        .validator_cardinality(storage, Timestamp::from_seconds(1 + unbonding_duration))
        .unwrap();
    let v1_after_unbond = st
        .validator_staked(storage, Timestamp::from_seconds(6), "v1".to_string())
        .unwrap();
    assert_eq!(v1_after_unbond, Uint128::zero());
    assert_eq!(cardinality_after_v1_unbond, 1);

    // @t=2, slash of all unbonding tokens for validator 1
    // cardinality reduced to 1 at t=2.
    st.on_unbonding_slash(
        storage,
        Timestamp::from_seconds(2),
        "v1".to_string(),
        delegation,
    )
    .unwrap();

    // check that cardinality, validator staked, and total staked now look as expected.
    let cardinality = st
        .validator_cardinality(storage, Timestamp::from_seconds(2))
        .unwrap();
    assert_eq!(cardinality, 1);
    let v1 = st
        .validator_staked(storage, Timestamp::from_seconds(2), "v1".to_string())
        .unwrap();
    assert_eq!(v1, Uint128::zero());

    // post-slash value remains zero.
    let v1 = st
        .validator_staked(storage, Timestamp::from_seconds(8), "v1".to_string())
        .unwrap();
    assert_eq!(v1, Uint128::zero());

    // @t=6, two more seconds of unbonding left for v2.
    let v2 = st
        .validator_staked(storage, Timestamp::from_seconds(6), "v2".to_string())
        .unwrap();
    assert_eq!(v2, delegation);
    let cardinality = st
        .validator_cardinality(storage, Timestamp::from_seconds(6))
        .unwrap();
    assert_eq!(cardinality, 1);

    // @t=8 all unbonding has completed.
    let v2 = st
        .validator_staked(storage, Timestamp::from_seconds(8), "v2".to_string())
        .unwrap();
    assert_eq!(v2, Uint128::zero());
    let v1 = st
        .validator_staked(storage, Timestamp::from_seconds(8), "v1".to_string())
        .unwrap();
    assert_eq!(v1, Uint128::zero());
    let cardinality = st
        .validator_cardinality(storage, Timestamp::from_seconds(8))
        .unwrap();
    assert_eq!(cardinality, 0);
}

/// Redelegating should cause cardinality changes if redelegation
/// removes all tokens from the source validator, or if it delegates
/// to a new validator.
#[test]
fn test_redelegation_changes_cardinality() {
    let storage = &mut mock_dependencies().storage;
    let st = StakeTracker::new("s", "v", "c");
    let t = Timestamp::default();
    let amount = Uint128::new(10);

    st.on_delegate(storage, t, "v1".to_string(), amount + amount)
        .unwrap();
    let c = st.validator_cardinality(storage, t).unwrap();
    assert_eq!(c, 1);

    st.on_redelegate(storage, t, "v1".to_string(), "v2".to_string(), amount)
        .unwrap();
    let c = st.validator_cardinality(storage, t).unwrap();
    assert_eq!(c, 2);

    st.on_redelegate(storage, t, "v1".to_string(), "v2".to_string(), amount)
        .unwrap();
    let c = st.validator_cardinality(storage, t).unwrap();
    assert_eq!(c, 1);
}

#[test]
fn test_queries() {
    let storage = &mut mock_dependencies().storage;
    let st = StakeTracker::new("s", "v", "c");
    st.on_delegate(
        storage,
        Timestamp::from_seconds(10),
        "v1".to_string(),
        Uint128::new(42),
    )
    .unwrap();

    let cardinality: Uint128 = from_json(
        st.query(
            storage,
            StakeTrackerQuery::Cardinality {
                t: Timestamp::from_seconds(11),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(cardinality, Uint128::one());

    let total_staked: Uint128 = from_json(
        st.query(
            storage,
            StakeTrackerQuery::TotalStaked {
                t: Timestamp::from_seconds(10),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(total_staked, Uint128::new(42));

    let val_staked: Uint128 = from_json(
        st.query(
            storage,
            StakeTrackerQuery::ValidatorStaked {
                t: Timestamp::from_seconds(10),
                validator: "v1".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(val_staked, Uint128::new(42));

    let val_staked_before_staking: Uint128 = from_json(
        st.query(
            storage,
            StakeTrackerQuery::ValidatorStaked {
                t: Timestamp::from_seconds(9),
                validator: "v1".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(val_staked_before_staking, Uint128::new(0));
}
