// Ignore integration tests for code coverage since there will be problems with dynamic linking libosmosistesttube
// and also, tarpaulin will not be able read coverage out of wasm binary anyway
#![cfg(not(tarpaulin))]

// Integrationg tests using an actual chain binary, requires
// the "test-tube" feature to be enabled
// cargo test --features test-tube
#[cfg(feature = "test-tube")]
pub mod cw_tokenfactory_issuer;
