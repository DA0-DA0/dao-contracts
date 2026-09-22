//! Native-only support for the pinned integration-test chain.
#![cfg(not(target_arch = "wasm32"))]

mod client;
pub mod config;
pub mod error;
pub mod gas;
pub mod registry;
pub mod rpc;
mod setup;
mod signing;

pub use client::ChainClient;
pub use config::{ChainConfig, Config};
pub use cosmrs::{AccountId as Address, Coin, Denom};
pub use error::{ProcessError, Result};
pub use setup::capture_setup_failure;
pub use signing::{Key, SigningKey};
