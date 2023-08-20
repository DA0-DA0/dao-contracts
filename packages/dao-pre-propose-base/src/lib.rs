#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod error;
pub mod execute;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;
