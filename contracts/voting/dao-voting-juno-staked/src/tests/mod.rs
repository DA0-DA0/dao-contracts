//! Unit tests for `dao-voting-juno-staked`.
//!
//! cw-multi-test has no first-class support for our `JunoQuery` custom
//! binding (the chain's `x/voting-snapshot` keeper isn't present in
//! integration test stubs), so we wire a small `MockQuerier`-derived
//! `JunoMockQuerier` that intercepts `QueryRequest::Custom(JunoQuery::..)`
//! and returns canned snapshot values. That's enough to exercise the
//! full query paths without needing a chain binary or multitest
//! module shim.

mod support;
mod surface;
