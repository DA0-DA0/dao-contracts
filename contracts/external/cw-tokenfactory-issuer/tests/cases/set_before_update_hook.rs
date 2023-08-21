use osmosis_test_tube::Account;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn test_set_before_update_hook_by_contract_owner() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
}

#[test]
fn test_set_before_update_hook_by_non_contract_owner_fails() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];
}
