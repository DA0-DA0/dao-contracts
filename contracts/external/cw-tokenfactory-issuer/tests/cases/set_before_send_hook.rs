use cosmwasm_std::coins;
use cw_tokenfactory_issuer::msg::QueryMsg;
use cw_tokenfactory_issuer::{state::BeforeSendHookInfo, ContractError};
use osmosis_test_tube::{Account, RunnerError};

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn test_set_before_send_hook() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    // Non-owner cannot set before update hook
    let err = env
        .cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );

    // Owner can set before update hook, but hook is already set
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Query before update hook
    let info: BeforeSendHookInfo = env
        .cw_tokenfactory_issuer
        .query(&QueryMsg::BeforeSendHookInfo {})
        .unwrap();
    assert!(info.advanced_features_enabled);
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn test_set_before_send_hook_nil() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // Owner can set before update hook to nil
    env.cw_tokenfactory_issuer
        .set_before_send_hook("".to_string(), owner)
        .unwrap();

    // Query before update hook, should now be disabled
    let info: BeforeSendHookInfo = env
        .cw_tokenfactory_issuer
        .query(&QueryMsg::BeforeSendHookInfo {})
        .unwrap();
    assert!(!info.advanced_features_enabled);
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn test_set_before_send_hook_invalid_address_fails() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // Invalid address fails
    let err = env
        .cw_tokenfactory_issuer
        .set_before_send_hook("invalid".to_string(), owner)
        .unwrap_err();

    assert_eq!(
        err,
        RunnerError::ExecuteError { msg: "failed to execute message; message index: 0: Generic error: addr_validate errored: decoding bech32 failed: invalid bech32 string length 7: execute wasm contract failed".to_string() }
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn test_set_before_send_hook_to_a_different_contract() {
    let env = TestEnv::default();
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let owner = &env.test_accs[0];
    let hook = &env.test_accs[1];

    // Owner can set before update hook to nil
    env.cw_tokenfactory_issuer
        .set_before_send_hook(hook.address(), owner)
        .unwrap();

    // Query before update hook, should now be disabled
    let info: BeforeSendHookInfo = env
        .cw_tokenfactory_issuer
        .query(&QueryMsg::BeforeSendHookInfo {})
        .unwrap();
    // Advanced features for this contract are not enabled
    assert!(!info.advanced_features_enabled);
    // But the hook contract address is set
    assert_eq!(info.hook_contract_address.unwrap(), hook.address());

    // Bank send should pass
    env.send_tokens(hook.address(), coins(10000, "uosmo"), owner)
        .unwrap();

    // Bank send of TF denom should fail as the hook account isn't a contract
    // and doesn't implement the required interface.
    env.send_tokens(hook.address(), coins(10000, denom), owner)
        .unwrap_err();
}
