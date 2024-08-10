// Integrationg tests using an actual chain binary, requires
// the "test-tube" feature to be enabled
// cargo test --features test-tube

#[cfg(feature = "test-tube")]
pub mod cw_admin_factory;

#[cfg(feature = "test-tube")]
pub mod cw_tokenfactory_issuer;

#[cfg(feature = "test-tube")]
pub mod cw4_group;

#[cfg(feature = "test-tube")]
pub mod cw721_base;

#[cfg(feature = "test-tube")]
pub mod dao_dao_core;

#[cfg(feature = "test-tube")]
pub mod dao_proposal_single;

#[cfg(feature = "test-tube")]
pub mod dao_test_custom_factory;

#[cfg(feature = "test-tube")]
pub mod dao_voting_cw4;
