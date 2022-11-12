# CosmWasm DAO Macros

This package provides a collection of macros that may be used to
derive DAO module interfaces on message enums. For example, to derive
the voting module interface on an enum:

```rust
#[token_query]
#[voting_query]
#[info_query]
#[cw_serde]
pub enum Query {}
```
