# CosmWasm JSONFilter

`cw-jsonfilter` is a Rust crate designed to facilitate filtering and comparing
JSON values based on specified criteria, specifically tailored for CosmWasm
message filtering. It provides functions for comparing JSON values, applying
filters to JSON objects, and determining if a filter matches a given JSON
object. Think of MongoDBs `find()` function but as a filter function. For a full
syntax guide, see the [Filter Documentation](./docs/filter.md).

This crate is forked from the
[jsonfilter](https://git.hydrar.de/jmarya/jsonfilter) repo, which uses the [MIT
license](./LICENSE-MIT). All modifications are licensed under the license used
by the broader dao-contracts repository, which is [BSD-3-Clause](../../LICENSE).

## Usage

To use `cw-jsonfilter`, add it to your `Cargo.toml` and add the following to your Rust code:

```rust
use cw_jsonfilter::{order, matches};
```

### Comparing JSON Values

You can compare two JSON values using the `order` function:

```rust
use serde_json::json;
use std::cmp::Ordering;
use cw_jsonfilter::order;

let a = json!(10);
let b = json!(5);
assert_eq!(order(&a, &b), Ordering::Greater);
```

### Matching Filters

To check if a JSON object matches a filter, use the `matches` function:

```rust
use serde_json::json;
use cw_jsonfilter::matches;

let filter = json!({"name": "John", "age": 30});
let obj = json!({"name": "John", "age": 30, "city": "New York"});

assert!(matches(&filter, &obj));
```
