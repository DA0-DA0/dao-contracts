use cw_tokenfactory_issuer::msg::QueryMsg;
use cw_tokenfactory_issuer::ContractError;
use osmosis_test_tube::RunnerError;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn test_set_before_send_hook() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    // Non-owner cannot set before update hook
    let err = env
        .cw_tokenfactory_issuer
        .set_before_send_hook(non_owner, env.cw_tokenfactory_issuer.contract_addr.clone())
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );

    // Owner can set before update hook, but hook is already set
    env.cw_tokenfactory_issuer
        .set_before_send_hook(owner, env.cw_tokenfactory_issuer.contract_addr.clone())
        .unwrap();

    // Query before update hook
    let enabled: bool = env
        .cw_tokenfactory_issuer
        .query(&QueryMsg::BeforeSendHookFeaturesEnabled {})
        .unwrap();
    assert!(enabled);
}

#[test]
fn test_set_before_send_hook_nil() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // Owner can set before update hook to nil
    env.cw_tokenfactory_issuer
        .set_before_send_hook(owner, "".to_string())
        .unwrap();

    // Query before update hook, should now be disabled
    let disabled: bool = env
        .cw_tokenfactory_issuer
        .query(&QueryMsg::BeforeSendHookFeaturesEnabled {})
        .unwrap();
    assert!(disabled);
}

#[test]
fn test_set_before_send_hook_invalid_address_fails() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // Invalid address fails
    let err = env
        .cw_tokenfactory_issuer
        .set_before_send_hook(owner, "invalid".to_string())
        .unwrap_err();

    assert_eq!(
        err,
        RunnerError::ExecuteError { msg: "failed to execute message; message index: 0: Generic error: addr_validate errored: decoding bech32 failed: invalid bech32 string length 7: execute wasm contract failed".to_string() }
    );
}
