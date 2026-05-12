use cosmwasm_std::{testing::mock_dependencies, Order, Uint128};

use crate::Wormhole;

#[test]
fn test_increment() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<String, Uint128> = Wormhole::new("ns");

    w.increment(storage, "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(), 10, Uint128::new(1))
        .unwrap();
    // incrementing 9 shoud cause the value at 10 to become 3
    w.increment(storage, "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(), 9, Uint128::new(2))
        .unwrap();

    assert_eq!(w.load(storage, "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(), 8).unwrap(), None);
    assert_eq!(
        w.load(storage, "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(), 9).unwrap(),
        Some(Uint128::new(2))
    );
    assert_eq!(
        w.load(storage, "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(), 10).unwrap(),
        Some(Uint128::new(3))
    );
}

#[test]
fn test_decrement() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<u64, i32> = Wormhole::new("ns");

    w.increment(storage, 1, 11, 4).unwrap();
    w.increment(storage, 1, 10, 10).unwrap();

    w.decrement(storage, 1, 9, 4).unwrap();

    assert_eq!(w.load(storage, 1, 8).unwrap(), None);
    assert_eq!(w.load(storage, 1, 9).unwrap(), Some(-4));
    assert_eq!(w.load(storage, 1, 10).unwrap(), Some(6));
    assert_eq!(w.load(storage, 1, 11).unwrap(), Some(10));
}

#[test]
fn test_load_matches_returned() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<(), u32> = Wormhole::new("ns");

    let v = w.increment(storage, (), 10, 10).unwrap();
    assert_eq!(v, w.load(storage, (), 10).unwrap().unwrap());

    let v = w.decrement(storage, (), 11, 1).unwrap();
    assert_eq!(v, w.load(storage, (), 11).unwrap().unwrap());
    assert_eq!(v, 9);
}

/// Calls to update should visit values in ascending order in terms of
/// time.
#[test]
fn test_update_visits_in_ascending_order() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<(), u32> = Wormhole::new("ns");

    w.increment(storage, (), 10, 10).unwrap();
    w.decrement(storage, (), 11, 1).unwrap();

    let mut seen = vec![];
    w.update(storage, (), 8, &mut |v, t| {
        seen.push((t, v));
        v
    })
    .unwrap();

    assert_eq!(seen, vec![(8, 0), (10, 10), (11, 9)])
}

/// Construct's the graph shown in the `dangerously_update` docstring
/// and verifies that the method behaves as expected.
#[test]
fn test_dangerous_update() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<(), u32> = Wormhole::new("ns");

    // (0) -> 20
    // (4) -> 10
    w.increment(storage, (), 0, 20).unwrap();
    w.decrement(storage, (), 4, 10).unwrap();

    // (3) -> 20
    let v = w.load(storage, (), 3).unwrap().unwrap();
    assert_eq!(v, 20);

    // (2) -> 15
    let also_v = w
        .dangerously_update(storage, (), 2, &mut |v, _| v - 5)
        .unwrap();

    // (2) -> 15
    let v = w.load(storage, (), 2).unwrap().unwrap();
    assert_eq!(v, 15);
    // check that returned value is same as loaded one.
    assert_eq!(also_v, 15);

    // (3) -> 15
    let v = w.load(storage, (), 3).unwrap().unwrap();
    assert_eq!(v, 15);

    // (4) -> 10, as dangerously_update should not change already set
    // values.
    let v = w.load(storage, (), 4).unwrap().unwrap();
    assert_eq!(v, 10);
}

/// Ensures redundant updates are removed by simulating a series of delegation
/// updates with auto-expiry in the future.
#[test]
fn test_redundant_updates_are_removed() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<(), u32> = Wormhole::new("ns");

    // Delegation expiry = 10
    //
    // Delegation updates:
    // - at block 0: +5 -> 5
    // - at block 2: +2 -> 7
    // - at block 5: -4 -> 3
    //
    // Each update undoes the previous auto-expiration, resetting the expiry.
    // Redundant values should be removed, meaning that by the end, the
    // underlying map should only have values at blocks 0, 2, 5, and 15. The
    // expiration blocks 10 and 12 that were reversed should be removed.

    // (0) -> +5 -> 5
    w.increment(storage, (), 0, 5).unwrap();
    assert_eq!(w.load(storage, (), 0).unwrap(), Some(5));
    // (10) -> -5 -> 0
    w.decrement(storage, (), 10, 5).unwrap();
    assert_eq!(w.load(storage, (), 10).unwrap(), Some(0));

    // (10) -> +5 -> 5
    w.increment(storage, (), 10, 5).unwrap();
    assert_eq!(w.load(storage, (), 10).unwrap(), Some(5));
    // (2) -> +2 -> 7
    // (10) -> 7
    w.increment(storage, (), 2, 2).unwrap();
    assert_eq!(w.load(storage, (), 2).unwrap(), Some(7));
    assert_eq!(w.load(storage, (), 10).unwrap(), Some(7));
    // (12) -> -7 -> 0
    w.decrement(storage, (), 12, 7).unwrap();
    assert_eq!(w.load(storage, (), 12).unwrap(), Some(0));

    // (12) -> +7 -> 7
    w.increment(storage, (), 12, 7).unwrap();
    assert_eq!(w.load(storage, (), 12).unwrap(), Some(7));
    // (5) -> -4 -> 3
    // (10) -> 3
    // (12) -> 3
    w.decrement(storage, (), 5, 4).unwrap();
    assert_eq!(w.load(storage, (), 5).unwrap(), Some(3));
    assert_eq!(w.load(storage, (), 10).unwrap(), Some(3));
    assert_eq!(w.load(storage, (), 12).unwrap(), Some(3));
    // (15) -> -3 -> 0
    w.decrement(storage, (), 15, 3).unwrap();
    assert_eq!(w.load(storage, (), 15).unwrap(), Some(0));

    // Final state:
    // (0) -> 5
    // (2) -> 7
    // (5) -> 3
    // (10) -> 3
    // (12) -> 3
    // (15) -> 0
    assert_eq!(w.load(storage, (), 0).unwrap(), Some(5));
    assert_eq!(w.load(storage, (), 2).unwrap(), Some(7));
    assert_eq!(w.load(storage, (), 5).unwrap(), Some(3));
    assert_eq!(w.load(storage, (), 10).unwrap(), Some(3));
    assert_eq!(w.load(storage, (), 12).unwrap(), Some(3));
    assert_eq!(w.load(storage, (), 15).unwrap(), Some(0));

    // Access underlying map to verify that only 0, 2, 5, and 15 are set.
    let mut iter = w
        .snapshots()
        .prefix(())
        .range(storage, None, None, Order::Ascending);
    assert_eq!(iter.next().unwrap().unwrap(), (0, 5));
    assert_eq!(iter.next().unwrap().unwrap(), (2, 7));
    assert_eq!(iter.next().unwrap().unwrap(), (5, 3));
    assert_eq!(iter.next().unwrap().unwrap(), (15, 0));
    assert_eq!(iter.next(), None);
}
