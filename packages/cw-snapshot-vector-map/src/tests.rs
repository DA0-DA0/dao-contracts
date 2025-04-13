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

#[test]
fn test_update() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );
    let k1 = &Addr::unchecked("bekauz");
    let item_1_value = 13;
    let item_2_value = 23;

    // push a couple of items: one with an expiration and one without
    let ((item_1_id, item_1_expiration), svm_size_1) =
        svm.push(storage, k1, &item_1_value, 1, None).unwrap();
    let ((item_2_id, item_2_expiration), svm_size_2) =
        svm.push(storage, k1, &item_2_value, 2, Some(5)).unwrap();

    assert_eq!(item_1_id, 0);
    assert_eq!(item_1_expiration, None);
    assert_eq!(svm_size_1, 1);
    let loaded_svm_item_1 = svm.load_item(storage, k1, item_1_id).unwrap();
    assert_eq!(loaded_svm_item_1, item_1_value);

    assert_eq!(item_2_id, 1);
    assert_eq!(item_2_expiration, Some(7));
    assert_eq!(svm_size_2, 2);
    let loaded_svm_item_2 = svm.load_item(storage, k1, item_2_id).unwrap();
    assert_eq!(loaded_svm_item_2, item_2_value);

    // perform an update of item1 at block #3, multiplying its value by 10
    let ((item_1_id_update1, item_1_expiration_update1), svm_size_3) = svm
        .update(storage, k1, item_1_id, 3, |v| *v *= 10, None)
        .unwrap();

    // assert that the snapshotvectormap size remains the same
    assert_eq!(svm_size_2, svm_size_3);
    // assert that the expiration of item1 remains the same (`None`)
    assert_eq!(item_1_expiration, item_1_expiration_update1);
    // assert that the newly assigned id is +1 from the last inserted item
    assert_eq!(item_1_id_update1, 2);

    // assert that at the current block values are unchanged
    assert_eq!(
        svm.load_all(storage, k1, 3).unwrap(),
        vec![
            LoadedItem {
                id: item_1_id,
                item: item_1_value,
                expiration: None
            },
            LoadedItem {
                id: item_2_id,
                item: item_2_value,
                expiration: Some(7)
            },
        ]
    );

    // now load values at the next block and assert that the previous item
    // with id `item_1_id` is gone and replaced by the new item with id `new_item_1_id`
    assert_eq!(
        svm.load_all(storage, k1, 4).unwrap(),
        vec![
            LoadedItem {
                id: item_2_id,
                item: item_2_value,
                expiration: Some(7)
            },
            LoadedItem {
                id: item_1_id_update1,
                item: item_1_value * 10,
                expiration: None
            }
        ]
    );

    // perform another update of item1 at block #4, multiplying its value by 2
    // and setting it to expire in 5 blocks
    let ((item_1_id_update2, item_1_expiration_update2), svm_size_4) = svm
        .update(storage, k1, item_1_id_update1, 4, |v| *v *= 2, Some(5))
        .unwrap();

    // assert that the snapshotvectormap size remains the same
    assert_eq!(svm_size_3, svm_size_4);
    // assert that the item now has an expiration at block height #9
    assert_eq!(item_1_expiration_update2, Some(9));
    // assert that the newly assigned id is +1 from the last inserted item
    assert_eq!(item_1_id_update2, 3);

    assert_eq!(
        svm.load_all(storage, k1, 5).unwrap(),
        vec![
            LoadedItem {
                id: item_2_id,
                item: item_2_value,
                expiration: Some(7)
            },
            LoadedItem {
                id: item_1_id_update2,
                item: item_1_value * 10 * 2,
                expiration: Some(9)
            }
        ]
    );

    // at block 6 both items should be included
    assert_eq!(svm.load_all(storage, k1, 6).unwrap().len(), 2);

    // at block 7 there should be one item remaining
    assert_eq!(svm.load_all(storage, k1, 7).unwrap().len(), 1);

    // at block 9 both items should be expired
    assert_eq!(svm.load_all(storage, k1, 9).unwrap().len(), 0);
}

#[test]
#[should_panic(expected = "update must be performed at or after the last update (1)")]
fn test_update_before_last_update() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );
    let k1 = &Addr::unchecked("bekauz");

    // push an item at block #1
    let ((item_id, _), _) = svm.push(storage, k1, &69, 1, None).unwrap();

    // attempt to update at block 0 (before last update at block 1)
    svm.update(storage, k1, item_id, 0, |v| *v += 1, None)
        .unwrap();
}

#[test]
#[should_panic(expected = "update must be performed at or after the last update (3)")]
fn test_update_in_past() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );
    let k1 = &Addr::unchecked("bekauz");

    // push an item at block #1
    let ((item_id, _), _) = svm.push(storage, k1, &69, 1, None).unwrap();

    // then push another item at block #3
    svm.push(storage, k1, &100, 3, None).unwrap();

    // attempts to update at block #2 should panic because there was
    // a push at block #3
    svm.update(storage, k1, item_id, 2, |v| *v += 1, None)
        .unwrap();
}

#[test]
#[should_panic]
fn test_update_non_existent_item() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );

    let k1 = &Addr::unchecked("bekauz");

    // attempt to update non-existent item when vector is empty
    svm.update(storage, k1, 0, 1, |v| *v += 1, None).unwrap();
}

#[test]
fn test_update_expired_item_creates_new_entry() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
        "svm__active__last_update",
    );
    let k1 = &Addr::unchecked("bekauz");

    // at block #1, push item that expires in 4 blocks (at block #5)
    let ((expired_id, _), _) = svm.push(storage, k1, &24, 1, Some(4)).unwrap();
    assert_eq!(expired_id, 0);

    // assert the expiration based on loading all items
    assert_eq!(svm.load_all(storage, k1, 4).unwrap().len(), 1);
    assert_eq!(svm.load_all(storage, k1, 5).unwrap().len(), 0);

    // update an expired item after its expiration will create a new entry
    let ((updated_id, _), _) = svm
        .update(storage, k1, expired_id, 6, |_| {}, Some(3))
        .unwrap();

    assert_eq!(svm.load_all(storage, k1, 6).unwrap().len(), 0);

    let next_block_active_list = svm.load_all(storage, k1, 7).unwrap();
    assert_eq!(next_block_active_list.len(), 1);
    assert_eq!(
        next_block_active_list[0],
        LoadedItem {
            id: updated_id,      // new id returned from the update call
            item: 24,            // same item value
            expiration: Some(9)  // new expiration
        }
    );
}
