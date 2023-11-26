# CosmWasm Map Pagination

This package provides generic convienence methods for paginating keys
and values in a CosmWasm `Map` or `SnapshotMap`. If you use these
methods to paginate the maps in your contract you may [make larry0x
happy](https://twitter.com/larry0x/status/1530537243709939719).

## Example

Given a map like:

```rust
use cw_storage_plus::Map;

pub const ITEMS: Map<String, String> = Map::new("items");
```

You can use this package to write a query to list it's contents like:

```rust
use cosmwasm_std::{Deps, Binary, to_json_binary, StdResult};
use cw_storage_plus::Map;
use cw_paginate_storage::paginate_map;                         

pub const ITEMS: Map<String, String> = Map::new("items");

pub fn query_list_items(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    to_json_binary(&paginate_map(
        deps,
        &ITEMS,
        start_after,
        limit,
        cosmwasm_std::Order::Descending,
    )?)
}
```
