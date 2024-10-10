use cosmwasm_std::{testing::mock_dependencies, Addr, BlockInfo, Timestamp};
use cw20::Expiration;
use cw_utils::Duration;

use crate::{LoadedItem, SnapshotVectorMap};

macro_rules! b {
    ($x:expr) => {
        &BlockInfo {
            chain_id: "".to_string(),
            height: $x,
            time: Timestamp::from_seconds($x),
        }
    };
}

#[test]
fn test_basic() {
    let storage = &mut mock_dependencies().storage;
    let svm: SnapshotVectorMap<Addr, u32> = SnapshotVectorMap::new(
        "svm__items",
        "svm__next_ids",
        "svm__active",
        "svm__active__checkpoints",
        "svm__active__changelog",
    );
    let k1 = &Addr::unchecked("haon");
    let k2 = &Addr::unchecked("ekez");

    // add 1, 2, 3 to k1 at corresponding blocks
    svm.push(storage, k1, &1, b!(1), None).unwrap();
    svm.push(storage, k1, &2, b!(2), None).unwrap();
    svm.push(storage, k1, &3, b!(3), None).unwrap();

    // add 1, 3 to k2 at corresponding blocks
    svm.push(storage, k2, &1, b!(1), None).unwrap();
    svm.push(storage, k2, &3, b!(3), None).unwrap();

    // items update one block later
    let items1_b2 = svm.load_all(storage, k1, b!(2)).unwrap();
    assert_eq!(
        items1_b2,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: None,
        }]
    );

    // items update one block later
    let items1_b4 = svm.load_all(storage, k1, b!(4)).unwrap();
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
    let items2_b3 = svm.load_all(storage, k2, b!(3)).unwrap();
    assert_eq!(
        items2_b3,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: None,
        }]
    );

    // remove item 2 (ID 1) from k1 at block 4
    svm.remove(storage, k1, 1, b!(4)).unwrap();

    // items update one block later
    let items1_b5 = svm.load_all(storage, k1, b!(5)).unwrap();
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
    );
    let k1 = &Addr::unchecked("haon");

    svm.push(storage, k1, &1, b!(1), Some(Duration::Height(3)))
        .unwrap();
    svm.push(storage, k1, &4, b!(4), None).unwrap();

    // items update one block later
    let items1_b2 = svm.load_all(storage, k1, b!(2)).unwrap();
    assert_eq!(
        items1_b2,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: Some(Expiration::AtHeight(4)),
        }]
    );

    // not expired yet
    let items1_b3 = svm.load_all(storage, k1, b!(3)).unwrap();
    assert_eq!(
        items1_b3,
        vec![LoadedItem {
            id: 0,
            item: 1,
            expiration: Some(Expiration::AtHeight(4)),
        }]
    );

    // expired:
    // load returns nothing
    let items1_b4 = svm.load_all(storage, k1, b!(4)).unwrap();
    assert_eq!(items1_b4, vec![]);
    // but vector still has item since the list hasn't been updated
    let active = svm
        .active
        .may_load_at_height(storage, k1.clone(), 4)
        .unwrap();
    assert_eq!(active, Some(vec![(0, Some(Expiration::AtHeight(4)))]));

    // new item exists now
    let items1_b5 = svm.load_all(storage, k1, b!(5)).unwrap();
    assert_eq!(
        items1_b5,
        vec![LoadedItem {
            id: 1,
            item: 4,
            expiration: None,
        }]
    );

    // add item that will expire
    svm.push(storage, k1, &5, b!(5), Some(Duration::Height(3)))
        .unwrap();

    let items1_b6 = svm.load_all(storage, k1, b!(6)).unwrap();
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
                expiration: Some(Expiration::AtHeight(8)),
            }
        ]
    );

    // removing first item at block 8 should expire the second item as well
    svm.remove(storage, k1, 1, b!(8)).unwrap();

    // load returns nothing (items update one block later)
    let items1_b9 = svm.load_all(storage, k1, b!(9)).unwrap();
    assert_eq!(items1_b9, vec![]);
    // and vector is empty since the remove updated the list
    let active = svm
        .active
        .may_load_at_height(storage, k1.clone(), 9)
        .unwrap();
    assert_eq!(active, Some(vec![]));

    // add item that will expire
    svm.push(storage, k1, &9, b!(9), Some(Duration::Height(2)))
        .unwrap();

    let items1_b10 = svm.load_all(storage, k1, b!(10)).unwrap();
    assert_eq!(
        items1_b10,
        vec![LoadedItem {
            id: 3,
            item: 9,
            expiration: Some(Expiration::AtHeight(11))
        }]
    );

    // push item at block 11, which should expire the existing item
    svm.push(storage, k1, &11, b!(11), None).unwrap();

    // load returns just the pushed item
    let items1_b12 = svm.load_all(storage, k1, b!(12)).unwrap();
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
}
