#[cfg(not(target_arch = "wasm32"))]
pub mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use tests::*;
