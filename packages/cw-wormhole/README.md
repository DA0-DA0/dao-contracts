# 🌀⏱️ CW Wormhole ⏱️🌀

A CosmWasm KV store that allows setting values from the past. For
example:

```rust
use cosmwasm_std::{testing::mock_dependencies, Uint128, Addr};
use cw_wormhole::Wormhole;
let storage = &mut mock_dependencies().storage;
let w: Wormhole<Addr, Uint128> = Wormhole::new("ns");
let key = Addr::unchecked("violet");

// increment the value by one at time 10.
w.increment(storage, key.clone(), 10, Uint128::new(1))
    .unwrap();

// increment the value by two at time 9.
w.increment(storage, key.clone(), 9, Uint128::new(2))
    .unwrap();

// the value at time 10 is now three.
assert_eq!(
    w.load(storage, key, 10).unwrap(),
    Some(Uint128::new(3))
);
```

Loading a value from the map is always constant time. Updating values
in the map is O(# future values). This has the effect of moving the
complexity of incrementing a future value into the present.

For a more in-depth analysis of the runtime of this data structure,
please see [this
essay](https://gist.github.com/0xekez/15fab6436ed593cbd59f0bdf7ecf1f61).

## Limitations

Reference types may not be used as keys.

Consider the trait bound:

```text
    for<'a> &'a (K, u64): PrimaryKey<'a>
```

This bound says, for any lifetime `'a` a reference to the tuple `(K,
u64)` will be a valid `PrimaryKey` with lifetime `'a`, thus we can
store tuples of this type in the map.

In order to allow K to have a lifetime (call it `'k`), we'd need to
write:

```text
    for<'a where 'a: 'k> &'a (K, u64): PrimaryKey<'a>
```

As the lifetime of the primary key is `'a + 'k` (the minimum of the
key's lifetime and the tuple's lifetime).

Unfourtunately, Rust does not support this. There is an RFC to
implement it
[here](https://github.com/tema3210/rfcs/blob/extended_hrtbs/text/3621-extended_hrtb.md).
