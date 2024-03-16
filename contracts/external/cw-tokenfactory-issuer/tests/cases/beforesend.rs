use cosmwasm_std::coins;
use osmosis_test_tube::{Account, RunnerError};

use crate::test_env::TestEnv;

#[test]
fn before_send_should_not_block_anything_by_default() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // Mint to self
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 10000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();

    // Bank send should pass
    env.send_tokens(env.test_accs[1].address(), coins(10000, denom), owner)
        .unwrap();
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn before_send_should_block_on_frozen() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Freeze
    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // Bank send should fail
    let err = env
        .send_tokens(
            env.test_accs[1].address(),
            coins(10000, denom.clone()),
            owner,
        )
        .unwrap_err();

    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The contract is frozen for denom \"{denom}\". Addresses need to be added to the allowlist to enable transfers to or from an account.: execute wasm contract failed") });
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn allowlisted_addresses_can_transfer_when_token_frozen() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let allowlistee = &env.test_accs[1];
    let other = &env.test_accs[2];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Mint to owner and allowlistee
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 100000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&allowlistee.address(), 10000, owner)
        .unwrap();

    // Freeze
    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // Allowlist address
    env.cw_tokenfactory_issuer
        .allow(&allowlistee.address(), true, owner)
        .unwrap();

    // Bank send should pass
    env.send_tokens(other.address(), coins(10000, denom.clone()), allowlistee)
        .unwrap();

    // Non allowlist address can't transfer, bank send should fail
    let err = env
        .send_tokens(other.address(), coins(10000, denom.clone()), owner)
        .unwrap_err();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The contract is frozen for denom \"{denom}\". Addresses need to be added to the allowlist to enable transfers to or from an account.: execute wasm contract failed") });

    // Other assets are not affected
    env.send_tokens(other.address(), coins(10000, "uosmo"), owner)
        .unwrap();
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn non_allowlisted_accounts_can_transfer_to_allowlisted_address_frozen() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let allowlistee = &env.test_accs[1];
    let other = &env.test_accs[2];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Mint to other
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 100000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&other.address(), 10000, owner)
        .unwrap();

    // Freeze
    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // Allowlist address
    env.cw_tokenfactory_issuer
        .allow(&allowlistee.address(), true, owner)
        .unwrap();

    // Bank send to allow listed address should pass
    env.send_tokens(allowlistee.address(), coins(10000, denom.clone()), other)
        .unwrap();
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn before_send_should_block_sending_from_denylist_address() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denylistee = &env.test_accs[1];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Mint to denylistee
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 20000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&denylistee.address(), 20000, owner)
        .unwrap();

    // Denylist
    env.cw_tokenfactory_issuer
        .deny(&denylistee.address(), true, owner)
        .unwrap();

    // Bank send should fail
    let err = env
        .send_tokens(
            env.test_accs[2].address(),
            coins(10000, denom.clone()),
            denylistee,
        )
        .unwrap_err();

    let denylistee_addr = denylistee.address();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The address '{denylistee_addr}' is denied transfer abilities: execute wasm contract failed") });
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn before_send_should_block_sending_to_denylist_address() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denylistee = &env.test_accs[1];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Mint to self
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 10000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();

    // Denylist
    env.cw_tokenfactory_issuer
        .deny(&denylistee.address(), true, owner)
        .unwrap();

    // Bank send should fail
    let err = env
        .send_tokens(denylistee.address(), coins(10000, denom.clone()), owner)
        .unwrap_err();

    let denylistee_addr = denylistee.address();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The address '{denylistee_addr}' is denied transfer abilities: execute wasm contract failed") });
}
