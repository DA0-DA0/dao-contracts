// Ignore integration tests for code coverage since there will be problems with dynamic linking libosmosistesttube
// and also, tarpaulin will not be able read coverage out of wasm binary anyway
#![cfg(not(tarpaulin))]

#[cfg(feature = "test-tube")]
mod cases;
#[cfg(feature = "test-tube")]
mod test_env;
