mod helpers;

use cosmwasm_std::coins;
use helpers::TestEnv;
use osmosis_testing::{Account, RunnerError};

#[test]
fn bank_send_should_be_allowed_by_default() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    // mint to self
    env.tokenfactory_issuer
        .set_minter(&owner.address(), 10000, owner)
        .unwrap();
    env.tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();

    // bank send should pass
    env.send_tokens(
        env.test_accs[1].address(),
        coins(10000, denom.clone()),
        owner,
    )
    .unwrap();
}

#[test]
fn frozen_contract_should_block_bank_send() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    // freeze
    env.tokenfactory_issuer
        .set_freezer(&owner.address(), true, owner)
        .unwrap();

    env.tokenfactory_issuer.freeze(true, owner).unwrap();

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
fn bank_send_from_blacklisted_address_should_be_blocked() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let blacklistee = &env.test_accs[1];
    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    // mint to blacklistee
    env.tokenfactory_issuer
        .set_minter(&owner.address(), 20000, owner)
        .unwrap();
    env.tokenfactory_issuer
        .mint(&blacklistee.address(), 20000, owner)
        .unwrap();

    // blacklist
    env.tokenfactory_issuer
        .set_blacklister(&owner.address(), true, owner)
        .unwrap();
    env.tokenfactory_issuer
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
fn bank_send_to_blacklisted_address_should_be_blocked() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let blacklistee = &env.test_accs[1];
    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    // mint to self
    env.tokenfactory_issuer
        .set_minter(&owner.address(), 10000, owner)
        .unwrap();
    env.tokenfactory_issuer
        .mint(&owner.address(), 10000, owner)
        .unwrap();

    // blacklist
    env.tokenfactory_issuer
        .set_blacklister(&owner.address(), true, owner)
        .unwrap();
    env.tokenfactory_issuer
        .blacklist(&blacklistee.address(), true, owner)
        .unwrap();

    // bank send should fail
    let err = env
        .send_tokens(blacklistee.address(), coins(10000, denom.clone()), owner)
        .unwrap_err();

    let blacklistee_addr = blacklistee.address();
    assert_eq!(err, RunnerError::ExecuteError { msg:  format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The address '{blacklistee_addr}' is blacklisted: execute wasm contract failed") });
}
