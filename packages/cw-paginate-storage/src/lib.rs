#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use cosmwasm_std::{Deps, Order, StdResult};

#[allow(unused_imports)]
use cw_storage_plus::{Bound, Bounder, KeyDeserialize, Map, SnapshotMap, Strategy};

/// Generic function for paginating a list of (K, V) pairs in a
/// CosmWasm Map.
pub fn paginate_map<'a, 'b, K, V, R: 'static>(
    deps: Deps,
    map: &Map<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<(R, V)>>
where
    K: Bounder<'a> + KeyDeserialize<Output = R> + 'b,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let (range_min, range_max) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };

    let items = map.range(deps.storage, range_min, range_max, order);
    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Same as `paginate_map` but only returns the keys.
pub fn paginate_map_keys<'a, 'b, K, V, R: 'static>(
    deps: Deps,
    map: &Map<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<R>>
where
    K: Bounder<'a> + KeyDeserialize<Output = R> + 'b,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let (range_min, range_max) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };

    let items = map.keys(deps.storage, range_min, range_max, order);
    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Same as `paginate_map` but for use with `SnapshotMap`.
pub fn paginate_snapshot_map<'a, 'b, K, V, R: 'static>(
    deps: Deps,
    map: &SnapshotMap<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<(R, V)>>
where
    K: Bounder<'a> + KeyDeserialize<Output = R> + 'b,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let (range_min, range_max) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };

    let items = map.range(deps.storage, range_min, range_max, order);
    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Same as `paginate_map` but only returns the values.
pub fn paginate_map_values<'a, K, V>(
    deps: Deps,
    map: &Map<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<V>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let (range_min, range_max) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };

    let items = map
        .range(deps.storage, range_min, range_max, order)
        .map(|kv| Ok(kv?.1));

    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Same as `paginate_map` but only returns the keys. For use with
/// `SnaphotMap`.
pub fn paginate_snapshot_map_keys<'a, 'b, K, V, R: 'static>(
    deps: Deps,
    map: &SnapshotMap<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<R>>
where
    K: Bounder<'a> + KeyDeserialize<Output = R> + 'b,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let (range_min, range_max) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };

    let items = map.keys(deps.storage, range_min, range_max, order);
    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{Addr, Uint128};

    #[test]
    fn pagination() {
        let mut deps = mock_dependencies();
        let map: Map<String, String> = Map::new("items");

        for num in 1..3 {
            map.save(&mut deps.storage, num.to_string(), &(num * 2).to_string())
                .unwrap();
        }

        let items = paginate_map(deps.as_ref(), &map, None, None, Order::Descending).unwrap();
        assert_eq!(
            items,
            vec![
                ("2".to_string(), "4".to_string()),
                ("1".to_string(), "2".to_string())
            ]
        );

        let items = paginate_map(deps.as_ref(), &map, None, None, Order::Ascending).unwrap();
        assert_eq!(
            items,
            vec![
                ("1".to_string(), "2".to_string()),
                ("2".to_string(), "4".to_string())
            ]
        );

        let items = paginate_map(
            deps.as_ref(),
            &map,
            Some("1".to_string()),
            None,
            Order::Ascending,
        )
        .unwrap();
        assert_eq!(items, vec![("2".to_string(), "4".to_string())]);

        let items = paginate_map(deps.as_ref(), &map, None, Some(1), Order::Ascending).unwrap();
        assert_eq!(items, vec![("1".to_string(), "2".to_string())]);
    }

    #[test]
    fn key_pagination() {
        let mut deps = mock_dependencies();
        let map: Map<String, String> = Map::new("items");

        for num in 1..3 {
            map.save(&mut deps.storage, num.to_string(), &(num * 2).to_string())
                .unwrap();
        }

        let items = paginate_map_keys(deps.as_ref(), &map, None, None, Order::Descending).unwrap();
        assert_eq!(items, vec!["2".to_string(), "1".to_string()]);

        let items = paginate_map_keys(deps.as_ref(), &map, None, None, Order::Ascending).unwrap();
        assert_eq!(items, vec!["1".to_string(), "2".to_string()]);

        let items = paginate_map_keys(
            deps.as_ref(),
            &map,
            Some("1".to_string()),
            None,
            Order::Ascending,
        )
        .unwrap();
        assert_eq!(items, vec!["2"]);

        let items =
            paginate_map_keys(deps.as_ref(), &map, None, Some(1), Order::Ascending).unwrap();
        assert_eq!(items, vec!["1".to_string()]);
    }

    // this test will double check the descending keys with the rewrite
    #[test]
    fn key_pagination_test2() {
        let mut deps = mock_dependencies();
        let map: Map<u32, String> = Map::new("items");

        for num in 1u32..=10 {
            map.save(&mut deps.storage, num, &(num * 2).to_string())
                .unwrap();
        }

        let items = paginate_map_keys(deps.as_ref(), &map, None, None, Order::Descending).unwrap();
        assert_eq!(items, vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);

        let items = paginate_map_keys(deps.as_ref(), &map, None, None, Order::Ascending).unwrap();
        assert_eq!(items, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let items =
            paginate_map_keys(deps.as_ref(), &map, Some(3), Some(3), Order::Ascending).unwrap();
        assert_eq!(items, vec![4, 5, 6]);

        let items =
            paginate_map_keys(deps.as_ref(), &map, Some(7), Some(4), Order::Descending).unwrap();
        assert_eq!(items, vec![6, 5, 4, 3]);
    }

    #[test]
    fn snapshot_pagination() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let map: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
            "items",
            "items__checkpoints",
            "items__changelog",
            Strategy::EveryBlock,
        );

        for ctr in 1..100 {
            let addr = Addr::unchecked(format!("test_addr{:0>3}", ctr.clone()));
            map.save(
                &mut deps.storage,
                &addr,
                &Uint128::new(ctr),
                env.block.height,
            )
            .unwrap();
        }

        // grab first 10 items
        let items =
            paginate_snapshot_map(deps.as_ref(), &map, None, Some(10), Order::Ascending).unwrap();

        assert_eq!(items.len(), 10);

        let mut test_vec: Vec<(Addr, Uint128)> = vec![];
        for ctr in 1..=10 {
            let addr = Addr::unchecked(format!("test_addr{:0>3}", ctr.clone()));

            test_vec.push((addr, Uint128::new(ctr)));
        }
        assert_eq!(items, test_vec);

        // using the last result of the last item (10), grab the next one
        let items = paginate_snapshot_map(
            deps.as_ref(),
            &map,
            Some(&items[items.len() - 1].0),
            Some(10),
            Order::Ascending,
        )
        .unwrap();

        // should be the 11th item
        assert_eq!(items[0].0, Addr::unchecked("test_addr011".to_string()));
        assert_eq!(items[0].1, Uint128::new(11));

        let items =
            paginate_snapshot_map(deps.as_ref(), &map, None, None, Order::Descending).unwrap();

        // 20th item (19 index) should be 80
        assert_eq!(items[19].0, Addr::unchecked("test_addr080".to_string()));
        assert_eq!(items[19].1, Uint128::new(80));
    }

    // this test will encapsulate the generic changes for &Addr
    #[test]
    fn snapshot_pagination_keys_new_generic() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let map: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
            "items",
            "items__checkpoints",
            "items__changelog",
            Strategy::EveryBlock,
        );

        for ctr in 1..100 {
            let addr = Addr::unchecked(format!("test_addr{:0>3}", ctr.clone()));
            map.save(
                &mut deps.storage,
                &addr,
                &Uint128::new(ctr),
                env.block.height,
            )
            .unwrap();
        }

        // grab first 10 items
        let items =
            paginate_snapshot_map_keys(deps.as_ref(), &map, None, Some(10), Order::Ascending)
                .unwrap();

        assert_eq!(items.len(), 10);

        let mut test_vec: Vec<Addr> = vec![];
        for ctr in 1..=10 {
            let addr = Addr::unchecked(format!("test_addr{:0>3}", ctr.clone()));

            test_vec.push(addr);
        }
        assert_eq!(items, test_vec);

        // max item from before was the 10th, so it'll go backwards from 9->1
        let items = paginate_snapshot_map_keys(
            deps.as_ref(),
            &map,
            Some(&items[items.len() - 1]),
            None,
            Order::Descending,
        )
        .unwrap();

        // 3rd item in vec should be 006
        assert_eq!(items[3], Addr::unchecked("test_addr006".to_string()));

        let items =
            paginate_snapshot_map_keys(deps.as_ref(), &map, None, None, Order::Descending).unwrap();

        // 20th item (19 index) should be 80
        assert_eq!(items[19], Addr::unchecked("test_addr080".to_string()));
    }

    #[test]
    fn snapshot_pagination_keys() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let map: SnapshotMap<u32, Uint128> = SnapshotMap::new(
            "items",
            "items__checkpoints",
            "items__changelog",
            Strategy::EveryBlock,
        );

        for ctr in 1..=100 {
            map.save(
                &mut deps.storage,
                ctr,
                &Uint128::new(ctr.try_into().unwrap()),
                env.block.height,
            )
            .unwrap();
        }

        // grab first 10 items
        let items =
            paginate_snapshot_map_keys(deps.as_ref(), &map, None, Some(10), Order::Ascending)
                .unwrap();

        assert_eq!(items.len(), 10);
        assert_eq!(items, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let items =
            paginate_snapshot_map_keys(deps.as_ref(), &map, Some(50), Some(10), Order::Ascending)
                .unwrap();

        assert_eq!(items, vec![51, 52, 53, 54, 55, 56, 57, 58, 59, 60]);

        let items =
            paginate_snapshot_map_keys(deps.as_ref(), &map, Some(50), Some(10), Order::Descending)
                .unwrap();

        assert_eq!(items, vec![49, 48, 47, 46, 45, 44, 43, 42, 41, 40]);
    }

    #[test]
    fn pagination_order_desc_tests() {
        let mut deps = mock_dependencies();
        let map: Map<u32, u32> = Map::new("items");

        map.save(&mut deps.storage, 1, &40).unwrap();
        map.save(&mut deps.storage, 2, &22).unwrap();
        map.save(&mut deps.storage, 3, &77).unwrap();
        map.save(&mut deps.storage, 4, &66).unwrap();
        map.save(&mut deps.storage, 5, &0).unwrap();

        let items = paginate_map(deps.as_ref(), &map, None, None, Order::Descending).unwrap();
        assert_eq!(items, vec![(5, 0), (4, 66), (3, 77), (2, 22), (1, 40)]);

        let items = paginate_map(deps.as_ref(), &map, Some(3), None, Order::Descending).unwrap();
        assert_eq!(items, vec![(2, 22), (1, 40)]);

        let items = paginate_map(deps.as_ref(), &map, Some(1), None, Order::Descending).unwrap();
        assert_eq!(items, vec![]);
    }

    /// testing reworked paginate_map and paginate_map_keys.
    /// pay particular attention to the values added. this is to ensure
    /// that the values arent being assessed
    #[test]
    fn pagination_keys_refs() {
        let mut deps = mock_dependencies();
        let map: Map<&Addr, u32> = Map::new("items");

        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 1)),
            &40,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 2)),
            &22,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 3)),
            &77,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 4)),
            &66,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 5)),
            &0,
        )
        .unwrap();

        let items = paginate_map_keys(deps.as_ref(), &map, None, None, Order::Descending).unwrap();
        assert_eq!(items[1], Addr::unchecked(format!("test_addr{:0>3}", 4)));
        assert_eq!(items[4], Addr::unchecked(format!("test_addr{:0>3}", 1)));

        let addr: Addr = Addr::unchecked(format!("test_addr{:0>3}", 3));
        let items =
            paginate_map_keys(deps.as_ref(), &map, Some(&addr), None, Order::Ascending).unwrap();
        assert_eq!(items[0], Addr::unchecked(format!("test_addr{:0>3}", 4)));
    }

    /// testing reworked paginate_map and paginate_map_keys.
    /// pay particular attention to the values added. this is to ensure
    /// that the values arent being assessed
    #[test]
    fn pagination_refs() {
        let mut deps = mock_dependencies();
        let map: Map<&Addr, u32> = Map::new("items");

        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 1)),
            &0,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 2)),
            &22,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 3)),
            &77,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 4)),
            &66,
        )
        .unwrap();
        map.save(
            &mut deps.storage,
            &Addr::unchecked(format!("test_addr{:0>3}", 6)),
            &0,
        )
        .unwrap();

        let items = paginate_map(deps.as_ref(), &map, None, None, Order::Descending).unwrap();
        assert_eq!(
            items[1],
            (Addr::unchecked(format!("test_addr{:0>3}", 4)), 66)
        );
        assert_eq!(
            items[4],
            (Addr::unchecked(format!("test_addr{:0>3}", 1)), 0)
        );

        let addr: Addr = Addr::unchecked(format!("test_addr{:0>3}", 3));
        let items =
            paginate_map(deps.as_ref(), &map, Some(&addr), Some(2), Order::Ascending).unwrap();
        let test_vec: Vec<(Addr, u32)> = vec![
            (Addr::unchecked(format!("test_addr{:0>3}", 4)), 66),
            (Addr::unchecked(format!("test_addr{:0>3}", 6)), 0),
        ];
        assert_eq!(items, test_vec);
    }
}
