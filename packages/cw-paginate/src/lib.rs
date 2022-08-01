//! # CosmWasm Map Pagination
//
//! This package provides generic convienence methods for paginating keys
//! and values in a CosmWasm `Maop` or `SnapshotMap`. If you use these
//! methods to paginate the maps in your contract you may [make larry0x
//! happy](https://twitter.com/larry0x/status/1530537243709939719).
//
//! ## Example
//
//! Given a map like:
//
//! ```rust
//! # use cw_storage_plus::Map;
//!
//! pub const ITEMS: Map<String, String> = Map::new("items");
//! ```
//
//! You can use this package to write a query to list it's contents like:
//
//! ```rust
//! # use cosmwasm_std::{Deps, Binary, to_binary, StdResult};
//! # use cw_storage_plus::Map;
//! # use cw_paginate::paginate_map;
//!
//! # pub const ITEMS: Map<String, String> = Map::new("items");
//!
//! pub fn query_list_items(
//!     deps: Deps,
//!     start_after: Option<String>,
//!     limit: Option<u32>,
//! ) -> StdResult<Binary> {
//!     to_binary(&paginate_map(
//!         deps,
//!         &ITEMS,
//!         start_after,
//!         limit,
//!         cosmwasm_std::Order::Descending,
//!     )?)
//! }
//!  ```

use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::{Bound, Bounder, KeyDeserialize, Map, SnapshotMap};

/// Generic function for paginating a list of (K, V) pairs in a
/// CosmWasm Map.
pub fn paginate_map<'a, K, V>(
    deps: Deps,
    map: &Map<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<(K, V)>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.range(deps.storage, start_after.map(Bound::exclusive), None, order);
    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Same as `paginate_map` but only returns the keys.
pub fn paginate_map_keys<'a, K, V>(
    deps: Deps,
    map: &Map<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<K>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.keys(deps.storage, start_after.map(Bound::exclusive), None, order);
    match limit {
        Some(limit) => Ok(items
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Same as `paginate_map` but only returns the keys. For use with
/// `SnaphotMap`.
pub fn paginate_snapshot_map_keys<'a, K, V>(
    deps: Deps,
    map: &SnapshotMap<'a, K, V>,
    start_after: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<K>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.keys(deps.storage, start_after.map(Bound::exclusive), None, order);
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
    use cosmwasm_std::testing::mock_dependencies;

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
}
