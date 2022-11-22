# CosmWasm DAO Macros

This package provides a collection of macros that may be used to
derive DAO module interfaces on message enums. For example, to derive
the voting module interface on an enum:

```rust
use cosmwasm_schema::{cw_serde, QueryResponses};
use cwd_macros::{token_query, voting_module_query};
use cwd_interface::voting::TotalPowerAtHeightResponse;
use cwd_interface::voting::VotingPowerAtHeightResponse;

#[token_query]
#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum Query {}
```
