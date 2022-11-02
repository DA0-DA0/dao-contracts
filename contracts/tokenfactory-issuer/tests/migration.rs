use osmosis_testing::Account;
use tokenfactory_issuer::ContractError;

mod helpers;
use helpers::{TestEnv, TokenfactoryIssuer};

#[test]
fn migrate_is_allowed_when_set_admin() {
    // setup
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // should be not frezon by default
    assert!(!env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen);

    // from tag `v0.1.0-migration-testdata`
    env.tokenfactory_issuer
        .migrate("tokenfactory_issuer_0.1.0_migration_testdata.wasm", owner)
        .unwrap();

    // frozen on migration
    assert!(env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen);
}
