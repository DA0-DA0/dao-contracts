use cosmwasm_std::{testing::mock_dependencies, Addr, Timestamp, Uint128};

use crate::stake_tracker::StakeTracker;

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
        st.validator_staked(storage, time, Addr::unchecked("v1"))
            .unwrap(),
        Uint128::zero()
    );

    // delegating increases validator cardinality, validator_staked, and total.
    st.on_delegate(storage, time, Addr::unchecked("v1"), Uint128::new(10))
        .unwrap();

    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 1);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::new(10));
    assert_eq!(
        st.validator_staked(storage, time, Addr::unchecked("v1"))
            .unwrap(),
        Uint128::new(10)
    );
    // delegating to one validator does not change the status of other validators.
    assert_eq!(
        st.validator_staked(storage, time, Addr::unchecked("v2"))
            .unwrap(),
        Uint128::zero()
    );

    // delegate to another validator, and undelegate from the first
    // one. the undelegation should not change cardinality or staked
    // values until the unbonding duration has passed.
    st.on_delegate(storage, time, Addr::unchecked("v2"), Uint128::new(10))
        .unwrap();
    st.on_undelegate(
        storage,
        time,
        Addr::unchecked("v1"),
        Uint128::new(10),
        unbonding_duration_seconds,
    )
    .unwrap();

    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 2);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::new(20));
    assert_eq!(
        st.validator_staked(storage, time, Addr::unchecked("v1"))
            .unwrap(),
        Uint128::new(10)
    );
    assert_eq!(
        st.validator_staked(storage, time, Addr::unchecked("v2"))
            .unwrap(),
        Uint128::new(10)
    );

    // after unbonding duration passes, undelegation changes should be
    // visible.
    time = time.plus_seconds(unbonding_duration_seconds);

    assert_eq!(st.validator_cardinality(storage, time).unwrap(), 1);
    assert_eq!(st.total_staked(storage, time).unwrap(), Uint128::new(10));
    assert_eq!(
        st.validator_staked(storage, time, Addr::unchecked("v1"))
            .unwrap(),
        Uint128::zero()
    );
    assert_eq!(
        st.validator_staked(storage, time, Addr::unchecked("v2"))
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
        Addr::unchecked("v2"),
        Uint128::new(10),
    )
    .unwrap();

    // there are 10 staked tokens total, but they are not staked to
    // this validator so removing them should cause an error.
    st.on_undelegate(
        storage,
        Timestamp::default(),
        Addr::unchecked("v1"),
        Uint128::new(10),
        10,
    )
    .unwrap();
}
