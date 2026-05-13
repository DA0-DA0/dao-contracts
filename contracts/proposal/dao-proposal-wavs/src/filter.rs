//! Wire-format types for querying a deployed `cw-filter` contract.
//!
//! We avoid importing the cw-filter crate as a direct dependency because its transitive
//! `cw-jsonfilter` pulls in `alloy-rpc-types-eth` whose internal types conflict at compile time.
//! Re-declaring the small wire surface here keeps us decoupled.
//!
//! Confirmed against `dao-contracts/contracts/external/cw-filter/src/msg.rs` 2026-05-08.
//! If the upstream wire format changes, update this file in lock-step.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::CosmosMsg;

/// Mirror of `cw_filter::msg::QueryMsg::Filter`. Only the discriminator we need.
#[cw_serde]
pub enum FilterQueryMsg {
    Filter {
        filter: serde_json::Value,
        msg: CosmosMsg,
    },
}

/// Mirror of `cw_filter::msg::FilterResponse`. Pass / Fail / Fatal.
#[cw_serde]
pub enum FilterResponse {
    Pass {},
    Fail { reason: String },
    Fatal { reason: String },
}
