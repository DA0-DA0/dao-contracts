mod helpers;
use helpers::TestEnv;
use osmosis_testing::{
    cosmrs::proto::cosmwasm::wasm::v1::{QueryContractInfoRequest, QueryContractInfoResponse},
    Runner,
};

#[test]
fn migrate_is_allowed_when_set_admin() {
    // setup
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let query_contract_info = || {
        env.app()
            .query::<QueryContractInfoRequest, QueryContractInfoResponse>(
                "/cosmwasm.wasm.v1.Query/ContractInfo",
                &QueryContractInfoRequest {
                    address: env.tokenfactory_issuer.contract_addr.clone(),
                },
            )
            .unwrap()
    };

    // should be not frezon by default
    assert!(!env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen);
    // first deployment should get code_id 1
    assert_eq!(query_contract_info().contract_info.unwrap().code_id, 1);

    // from tag `v0.1.0-migration-testdata`
    env.tokenfactory_issuer
        .migrate("tokenfactory_issuer_0.1.0_migration_testdata.wasm", owner)
        .unwrap();

    // frozen on migration
    assert!(env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen);
    // migration should set code_id 2
    assert_eq!(query_contract_info().contract_info.unwrap().code_id, 2);
}
