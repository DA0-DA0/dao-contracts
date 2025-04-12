use cosmwasm_std::{testing::mock_dependencies, Addr, StdError};

use crate::{LoadedItem, SnapshotVectorMap};

#[test]
fn test_basic() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );
    let k1 = &Addr::unchecked("haon");
    let k2 = &Addr::unchecked("ekez");

    // add 1, 2, 3 to k1 at corresponding blocks
    svm.push(storage, k1, &1, 1, None).unwrap();
    svm.push(storage, k1, &2, 2, None).unwrap();
    svm.push(storage, k1, &3, 3, None).unwrap();

    // add 1, 3 to k2 at corresponding blocks
    svm.push(storage, k2, &1, 1, None).unwrap();
    svm.push(storage, k2, &3, 3, None).unwrap();

    // items update one block later
    let items1_b2 = svm.load_all(storage, k1, 2).unwrap();
    assert_eq!(
        items1_b2,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: None,
        }]
    );

    // items update one block later
    let items1_b4 = svm.load_all(storage, k1, 4).unwrap();
    assert_eq!(
        items1_b4,
        vec![
            LoadedItem {
                id: 0,
                item: 1,
                expiration: None,
            },
            LoadedItem {
                id: 1,
                item: 2,
                expiration: None,
            },
            LoadedItem {
                id: 2,
                item: 3,
                expiration: None,
            }
        ]
    );

    // items update one block later
    let items2_b3 = svm.load_all(storage, k2, 3).unwrap();
    assert_eq!(
        items2_b3,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: None,
        }]
    );

    // remove item 2 (ID 1) from k1 at block 4
    svm.remove(storage, k1, 1, 4).unwrap();

    // items update one block later
    let items1_b5 = svm.load_all(storage, k1, 5).unwrap();
    assert_eq!(
        items1_b5,
        vec![
            LoadedItem {
                id: 0,
                item: 1,
                expiration: None,
            },
            LoadedItem {
                id: 2,
                item: 3,
                expiration: None,
            }
        ]
    );

    // cannot push in the past
    for i in 0..4 {
        let err = svm.push(storage, k1, &4, i, None).unwrap_err();
        assert_eq!(
            err,
            StdError::generic_err("update must be performed at or after the last update (4)")
        );
    }

    // can push at the same height as the last update (block 4)
    let ((added_id, _), _) = svm.push(storage, k1, &4, 4, None).unwrap();

    // cannot remove in the past
    for i in 0..4 {
        let err = svm.remove(storage, k1, 0, i).unwrap_err();
        assert_eq!(
            err,
            StdError::generic_err("update must be performed at or after the last update (4)")
        );
    }

    // can remove at the same height as the last update
    // remove item we just added from k1 at block 4
    svm.remove(storage, k1, added_id, 4).unwrap();
}

#[test]
fn test_expiration() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );
    let k1 = &Addr::unchecked("haon");

    svm.push(storage, k1, &1, 1, Some(3)).unwrap();
    svm.push(storage, k1, &4, 4, None).unwrap();

    // items update one block later
    let items1_b2 = svm.load_all(storage, k1, 2).unwrap();
    assert_eq!(
        items1_b2,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: Some(4),
        }]
    );

    // not expired yet
    let items1_b3 = svm.load_all(storage, k1, 3).unwrap();
    assert_eq!(
        items1_b3,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: Some(4),
        }]
    );

    // expired:
    // load returns nothing
    let items1_b4 = svm.load_all(storage, k1, 4).unwrap();
    assert_eq!(items1_b4, vec![]);
    // but vector still has item since the list hasn't been updated
    let active = svm
        .active
        .may_load_at_height(storage, k1.clone(), 4)
        .unwrap();
    assert_eq!(active, Some(vec![(0, Some(4))]));

    // new item exists now
    let items1_b5 = svm.load_all(storage, k1, 5).unwrap();
    assert_eq!(
        items1_b5,
        vec![LoadedItem {
            id: 1,
            item: 4,
            expiration: None,
        }]
    );

    // add item that will expire
    svm.push(storage, k1, &5, 5, Some(3)).unwrap();

    let items1_b6 = svm.load_all(storage, k1, 6).unwrap();
    assert_eq!(
        items1_b6,
        vec![
            LoadedItem {
                id: 1,
                item: 4,
                expiration: None
            },
            LoadedItem {
                id: 2,
                item: 5,
                expiration: Some(8),
            }
        ]
    );

    // removing first item at block 8 should expire the second item as well
    svm.remove(storage, k1, 1, 8).unwrap();

    // load returns nothing (items update one block later)
    let items1_b9 = svm.load_all(storage, k1, 9).unwrap();
    assert_eq!(items1_b9, vec![]);
    // and vector is empty since the remove updated the list
    let active = svm
        .active
        .may_load_at_height(storage, k1.clone(), 9)
        .unwrap();
    assert_eq!(active, Some(vec![]));

    // add item that will expire
    svm.push(storage, k1, &9, 9, Some(2)).unwrap();

    let items1_b10 = svm.load_all(storage, k1, 10).unwrap();
    assert_eq!(
        items1_b10,
        vec![LoadedItem {
            id: 3,
            item: 9,
            expiration: Some(11)
        }]
    );

    // push item at block 11, which should expire the existing item
    svm.push(storage, k1, &11, 11, None).unwrap();

    // load returns just the pushed item
    let items1_b12 = svm.load_all(storage, k1, 12).unwrap();
    assert_eq!(
        items1_b12,
        vec![LoadedItem {
            id: 4,
            item: 11,
            expiration: None,
        }]
    );
    // and vector only contains the pushed item since remove updated the list
    let active = svm
        .active
        .may_load_at_height(storage, k1.clone(), 12)
        .unwrap();
    assert_eq!(active, Some(vec![(4, None)]));

    // add item 13 that will expire in 10 blocks and 14 that will expire in 5
    // blocks
    svm.push(storage, k1, &13, 13, Some(10)).unwrap();
    svm.push(storage, k1, &14, 14, Some(5)).unwrap();

    let items1_b15 = svm.load_all(storage, k1, 15).unwrap();
    assert_eq!(
        items1_b15,
        vec![
            LoadedItem {
                id: 4,
                item: 11,
                expiration: None,
            },
            LoadedItem {
                id: 5,
                item: 13,
                expiration: Some(23),
            },
            LoadedItem {
                id: 6,
                item: 14,
                expiration: Some(19),
            }
        ]
    );

    // update expiration of item 5 to block 26
    svm.update_expiration(storage, k1, 5, 16, Some(10)).unwrap();

    // expiration update should take effect immediately
    let items1_b16 = svm.load_all(storage, k1, 16).unwrap();
    assert_eq!(
        items1_b16,
        vec![
            LoadedItem {
                id: 4,
                item: 11,
                expiration: None,
            },
            LoadedItem {
                id: 5,
                item: 13,
                expiration: Some(26),
            },
            LoadedItem {
                id: 6,
                item: 14,
                expiration: Some(19),
            }
        ]
    );

    // update expiration of item 5 to block 30 at block 19
    svm.update_expiration(storage, k1, 5, 19, Some(11)).unwrap();

    // expiration update should take effect immediately, and item 6 should be
    // removed since it expired
    let items1_b19 = svm.load_all(storage, k1, 19).unwrap();
    assert_eq!(
        items1_b19,
        vec![
            LoadedItem {
                id: 4,
                item: 11,
                expiration: None,
            },
            LoadedItem {
                id: 5,
                item: 13,
                expiration: Some(30),
            },
        ]
    );

    let items1_b30 = svm.load_all(storage, k1, 30).unwrap();
    assert_eq!(
        items1_b30,
        vec![LoadedItem {
            id: 4,
            item: 11,
            expiration: None,
        }]
    );

    // update expiration of item 4 to block 35
    svm.update_expiration(storage, k1, 4, 30, Some(5)).unwrap();

    // expiration update should take effect immediately
    let items1_b35 = svm.load_all(storage, k1, 30).unwrap();
    assert_eq!(
        items1_b35,
        vec![LoadedItem {
            id: 4,
            item: 11,
            expiration: Some(35),
        }]
    );

    // expiration extensions should back-propagate to last time an item was
    // added/removed
    let items1_b15 = svm.load_all(storage, k1, 15).unwrap();
    assert_eq!(
        items1_b15,
        vec![
            LoadedItem {
                id: 4,
                item: 11,
                expiration: Some(35),
            },
            LoadedItem {
                id: 5,
                item: 13,
                expiration: Some(30),
            },
            LoadedItem {
                id: 6,
                item: 14,
                expiration: Some(19),
            }
        ]
    );

    // add item 7 that will expire in 10 blocks
    svm.push(storage, k1, &31, 31, Some(10)).unwrap();

    let items1_b31 = svm.load_all(storage, k1, 31).unwrap();
    assert_eq!(
        items1_b31,
        vec![LoadedItem {
            id: 4,
            item: 11,
            expiration: Some(35),
        }]
    );

    let items1_b32 = svm.load_all(storage, k1, 32).unwrap();
    assert_eq!(
        items1_b32,
        vec![
            LoadedItem {
                id: 4,
                item: 11,
                expiration: Some(35),
            },
            LoadedItem {
                id: 7,
                item: 31,
                expiration: Some(41),
            }
        ]
    );

    // update expiration of item 4 to block 40 and item 7 to block 45
    svm.update_expiration(storage, k1, 4, 33, Some(7)).unwrap();
    svm.update_expiration(storage, k1, 7, 33, Some(12)).unwrap();

    // expiration update should take effect immediately
    let items1_b33 = svm.load_all(storage, k1, 33).unwrap();
    assert_eq!(
        items1_b33,
        vec![
            LoadedItem {
                id: 4,
                item: 11,
                expiration: Some(40),
            },
            LoadedItem {
                id: 7,
                item: 31,
                expiration: Some(45),
            }
        ]
    );

    // expiration extensions should back-propagate to last time an item was
    // added/removed
    let items1_b32 = svm.load_all(storage, k1, 32).unwrap();
    assert_eq!(items1_b32, items1_b33);

    // cannot update expiration before the last update
    let err = svm
        .update_expiration(storage, k1, 4, 30, Some(10))
        .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("update must be performed at or after the last update (31)")
    );
}
