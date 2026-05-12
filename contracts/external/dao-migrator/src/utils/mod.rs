// query_helpers + state_queries hold v1 -> v2 type-conversion + query helpers for
// the v1 migrator path. Gated off because cosmwasm-std 1.x and 2.x produce
// distinct `Decimal` / `Uint128` / `Timestamp` / `Addr` / `CosmosMsg` types
// across the version boundary; bridging them needs a separate shim design.
// Re-enable once the v1 -> v2.9+ shim lands.
#[cfg(any())]
pub mod query_helpers;
#[cfg(any())]
pub mod state_queries;
