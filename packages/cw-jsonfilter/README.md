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

To use `cw-jsonfilter`, add it to your `Cargo.toml` and add the following to
your Rust code:

```rust
use cw_jsonfilter::CwJsonFilter;
```

### Filtering

To check if a JSON object matches a filter, use the `CwJsonFilter` struct:

```rust
use serde_json::json;
use cw_jsonfilter::CwJsonFilter;

let filter = json!({"name": "John", "age": 30});
let obj = json!({"name": "John", "age": 30, "city": "New York"});

assert!(CwJsonFilter::check(&filter, &obj).is_pass());
```

### Protobuf

This crate supports complex protobuf decoding and filtering. See the [Protobuf
Documentation](./docs/protobuf.md) for more details.

## Notes

### Error Types: Failure vs Fatal

There are two types of errors that can occur when checking a filter:

- **Failure**: The filter failed due to a value not passing the filter. This is
  either an operator failure (which includes implicit equality checks) or a key
  not found.
- **Fatal**: The filter encountered a fatal error due to a malformed filter, not
  dependent on the value passed in whatsoever. This is either an unknown
  operator or an invalid filter (which typically means the filter argument(s)
  are incorrect).

This distinction is relevant because fatal errors should halt processing of the
entire filter chain, whereas failure errors should just be considered a test
failure that the parent operator may need to handle in a specific way.

For example, the $xor operator requires exactly one subfilter to pass, meaning
the rest must fail. Typical failures should not halt the filter chain since they
are expected and in fact necessary, as long as one passes. However, if a filter
is malformed, meaning that its arguments are invalid, we cannot be sure of the
intended logical outcome. Thus we cannot proceed with the filter chain because
it may result in a false negative or positive.

This is especially clear when examining the $not operator, which flips the
typical pass/fail logic. If its subfilter passes, the $not operator should fail,
and vice versa. But, crucially, if the subfilter fails due to a misspelled
operator, we may receive a false negative, which would result in a false
positive if the $not operator were to treat it as a valid and expected failure.

Thus, a fatal error should halt the filter chain to prevent operators from
mistakingly treating them as valid failures and allowing something to pass that
should not have.

The distinction can also be understood as:

- **Failure**: A valid filter with proper arguments that fails because the value
  does not match the expected shape.
- **Fatal**: An error that occurs regardless of the value passed in, meaning
  that the filter is malformed.

If a different value can lead to a different outcome, then it is not fatal—it is
simply a failure. If a different value has no impact on the error, then it is
fatal.
