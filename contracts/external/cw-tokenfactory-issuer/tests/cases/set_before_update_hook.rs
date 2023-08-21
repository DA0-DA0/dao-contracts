use cw_tokenfactory_issuer::ContractError;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn test_set_before_update_hook() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    // Non-owner cannot set before update hook
    let err = env
        .cw_tokenfactory_issuer
        .set_before_send_hook(non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );

    // Owner can set before update hook
    env.cw_tokenfactory_issuer
        .set_before_send_hook(owner)
        .unwrap();
}
