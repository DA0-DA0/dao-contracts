# CosmWasm DAO Macros

This package provides a collection of macros that may be used to
derive DAO module interfaces on message enums. For example, to derive
the voting module interface on an enum:

```rust
#[token_query]
#[voting_query]
#[info_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Query {}
```
