//! Juno-specific custom wasm bindings consumed by this contract.
//!
//! These types mirror the JSON shape produced by
//! `juno/wasmbindings/types/query.go` byte-for-byte. The chain registers a
//! `QueryPlugin` that dispatches based on the tagged variant; anything that
//! doesn't match results in a wasm query error.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CustomQuery, QuerierWrapper, QueryRequest, StdResult, Uint128};

/// Top-level custom query enum. Variants are tagged exactly as the chain
/// expects (lowercase snake_case via `cw_serde`).
#[cw_serde]
#[derive(Eq)]
pub enum JunoQuery {
    /// Bonded voting power for `address` at-or-before `height`, excluding
    /// addresses on Juno's governance-managed LST allowlist. The chain returns
    /// the most recent snapshot whose recorded height is `<= height`; zero if
    /// no snapshot exists.
    VotingPowerAt(VotingPowerAt),
    /// Total bonded voting power at-or-before `height`.
    TotalVotingPowerAt(TotalVotingPowerAt),
    /// Every recorded snapshot for `address` in `[from_height, to_height]`.
    /// Not used by the voting module itself but exposed here so consumers
    /// in this workspace can share the binding types.
    VotingPowerOverRange(VotingPowerOverRange),
}

impl CustomQuery for JunoQuery {}

#[cw_serde]
#[derive(Eq)]
pub struct VotingPowerAt {
    pub address: String,
    pub height: i64,
}

#[cw_serde]
#[derive(Eq)]
pub struct TotalVotingPowerAt {
    pub height: i64,
}

#[cw_serde]
#[derive(Eq)]
pub struct VotingPowerOverRange {
    pub address: String,
    pub from_height: i64,
    pub to_height: i64,
}

/// Response for both `VotingPowerAt` and `TotalVotingPowerAt`. The chain
/// emits the bonded amount as a base-10 string sized for uint256 — we
/// parse to `Uint128` and surface overflow as a query error.
#[cw_serde]
pub struct VotingPowerResponse {
    pub power: String,
}

#[cw_serde]
pub struct VotingPowerOverRangeResponse {
    pub rows: Vec<HeightPower>,
}

#[cw_serde]
pub struct HeightPower {
    pub height: i64,
    pub power: String,
}

/// Helpers wrapping the raw `QuerierWrapper<JunoQuery>` so call sites read
/// like ordinary querier calls.
pub trait JunoQuerier {
    fn voting_power_at(&self, address: String, height: u64) -> StdResult<Uint128>;
    fn total_voting_power_at(&self, height: u64) -> StdResult<Uint128>;
}

impl<'a> JunoQuerier for QuerierWrapper<'a, JunoQuery> {
    fn voting_power_at(&self, address: String, height: u64) -> StdResult<Uint128> {
        let height = query_height(height)?;
        let req: QueryRequest<JunoQuery> =
            QueryRequest::Custom(JunoQuery::VotingPowerAt(VotingPowerAt { address, height }));
        let resp: VotingPowerResponse = self.query(&req)?;
        parse_power(&resp.power)
    }

    fn total_voting_power_at(&self, height: u64) -> StdResult<Uint128> {
        let height = query_height(height)?;
        let req: QueryRequest<JunoQuery> =
            QueryRequest::Custom(JunoQuery::TotalVotingPowerAt(TotalVotingPowerAt { height }));
        let resp: VotingPowerResponse = self.query(&req)?;
        parse_power(&resp.power)
    }
}

fn query_height(height: u64) -> StdResult<i64> {
    i64::try_from(height).map_err(|_| {
        cosmwasm_std::StdError::generic_err(format!(
            "voting-snapshot query height {height} exceeds i64::MAX"
        ))
    })
}

fn parse_power(raw: &str) -> StdResult<Uint128> {
    raw.parse::<Uint128>().map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!(
            "voting-snapshot returned unparseable power '{raw}': {e}"
        ))
    })
}
