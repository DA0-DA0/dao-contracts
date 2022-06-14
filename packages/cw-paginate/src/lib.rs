use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::{Bound, Bounder, KeyDeserialize, Map};

/// Generic function for paginating a list of (K, V) pairs in a
/// CosmWasm Map.
pub fn paginate_map<'a, K, V>(
    deps: Deps,
    map: &Map<'a, K, V>,
    start_at: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<(K, V)>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.range(deps.storage, start_at.map(Bound::inclusive), None, order);
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
    start_at: Option<K>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<K>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.keys(deps.storage, start_at.map(Bound::inclusive), None, order);
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
            Some("2".to_string()),
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
            Some("2".to_string()),
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
