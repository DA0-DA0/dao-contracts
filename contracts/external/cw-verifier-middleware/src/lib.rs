#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod error;
pub mod msg;
pub mod state;
pub mod utils;
pub mod verify;

#[cfg(test)]
mod testing;
