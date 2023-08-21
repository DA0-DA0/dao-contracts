#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

#[cfg(not(target_arch = "wasm32"))]
pub mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod helpers;

#[cfg(not(target_arch = "wasm32"))]
pub mod contracts;

#[cfg(not(target_arch = "wasm32"))]
pub use tests::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_tube;
