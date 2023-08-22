use cosmwasm_std::coins;
use osmosis_test_tube::{Account, RunnerError};

use crate::test_env::TestEnv;

#[test]
fn before_send_should_not_block_anything_by_default() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // mint to self
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 10000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();

    // bank send should pass
    env.send_tokens(env.test_accs[1].address(), coins(10000, denom), owner)
        .unwrap();
}

#[test]
fn before_send_should_block_on_frozen() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // freeze
    env.cw_tokenfactory_issuer
        .set_freezer(&owner.address(), true, owner)
        .unwrap();

    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // bank send should fail
    let err = env
        .send_tokens(
            env.test_accs[1].address(),
            coins(10000, denom.clone()),
            owner,
        )
        .unwrap_err();

    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The contract is frozen for denom \"{denom}\": execute wasm contract failed") });
}

#[test]
fn white_listed_addresses_can_transfer_when_token_frozen() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let whitelistee = &env.test_accs[1];
    let other = &env.test_accs[2];

    // freeze
    env.cw_tokenfactory_issuer
        .set_freezer(&owner.address(), true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // bank send should fail
    let err = env
        .send_tokens(whitelistee.address(), coins(10000, denom.clone()), owner)
        .unwrap_err();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The contract is frozen for denom \"{denom}\": execute wasm contract failed") });

    // Whitelist address
    env.cw_tokenfactory_issuer
        .set_whitelister(&owner.address(), true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .whitelist(&whitelistee.address(), true, owner)
        .unwrap();

    // bank send should pass
    env.send_tokens(other.address(), coins(10000, denom.clone()), whitelistee)
        .unwrap_err();
    // Non whitelisted address can't transfer, bank send should fail
    let err = env
        .send_tokens(other.address(), coins(10000, denom.clone()), owner)
        .unwrap_err();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The contract is frozen for denom \"{denom}\": execute wasm contract failed") });
}

#[test]
fn before_send_should_block_sending_from_blacklisted_address() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let blacklistee = &env.test_accs[1];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // mint to blacklistee
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 20000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&blacklistee.address(), 20000, owner)
        .unwrap();

    // blacklist
    env.cw_tokenfactory_issuer
        .set_blacklister(&owner.address(), true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .blacklist(&blacklistee.address(), true, owner)
        .unwrap();

    // bank send should fail
    let err = env
        .send_tokens(
            env.test_accs[2].address(),
            coins(10000, denom.clone()),
            blacklistee,
        )
        .unwrap_err();

    let blacklistee_addr = blacklistee.address();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The address '{blacklistee_addr}' is blacklisted: execute wasm contract failed") });
}

#[test]
fn before_send_should_block_sending_to_blacklisted_address() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let blacklistee = &env.test_accs[1];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    // mint to self
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), 10000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();

    // blacklist
    env.cw_tokenfactory_issuer
        .set_blacklister(&owner.address(), true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .blacklist(&blacklistee.address(), true, owner)
        .unwrap();

    // bank send should fail
    let err = env
        .send_tokens(blacklistee.address(), coins(10000, denom.clone()), owner)
        .unwrap_err();

    let blacklistee_addr = blacklistee.address();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The address '{blacklistee_addr}' is blacklisted: execute wasm contract failed") });
}
