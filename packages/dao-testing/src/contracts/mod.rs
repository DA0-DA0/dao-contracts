mod latest;

// `v1` and `v241` ship stub implementations under this binary: the upstream
// v1.x / v2.4.1 contract crates pin cosmwasm-std 1.5.5, incompatible with
// cw-multi-test 2.x's ContractWrapper. The stubs let test crates that import
// these wrappers keep compiling; calls into them error out at runtime.
// Re-implement with real v1.x / v2.4.1 behaviour once the v1 -> v2.9+
// migration shim lands.
pub mod v1;
pub mod v241;

pub use latest::*;
