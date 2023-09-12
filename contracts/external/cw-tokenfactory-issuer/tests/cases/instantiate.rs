use cosmwasm_std::Addr;
use cw_tokenfactory_issuer::{
    msg::{InstantiateMsg, QueryMsg},
    state::BeforeSendHookInfo,
};
use osmosis_test_tube::{Account, OsmosisTestApp};

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn instantiate_with_new_token_should_set_initial_state_correctly() {
    let subdenom = "uthb".to_string();
    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom: subdenom.clone(),
        },
        0,
    )
    .unwrap();

    let owner = &env.test_accs[0];

    // Check tokenfactory's token admin
    let denom = format!(
        "factory/{}/{}",
        env.cw_tokenfactory_issuer.contract_addr, subdenom
    );

    assert_eq!(
        env.token_admin(&denom),
        env.cw_tokenfactory_issuer.contract_addr,
        "token admin must be tokenfactory-issuer contract"
    );

    // Check initial contract state
    let contract_denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    assert_eq!(
        denom, contract_denom,
        "denom stored in contract must be `factory/<contract_addr>/<subdenom>`"
    );

    // Contract is not frozen
    let is_frozen = env
        .cw_tokenfactory_issuer
        .query_is_frozen()
        .unwrap()
        .is_frozen;
    assert!(!is_frozen, "newly instantiated contract must not be frozen");

    // Advanced features requiring BeforeSendHook are disabled
    let info: BeforeSendHookInfo = env
        .cw_tokenfactory_issuer
        .query(&QueryMsg::BeforeSendHookInfo {})
        .unwrap();
    assert!(!info.advanced_features_enabled);

    let owner_addr = env.cw_tokenfactory_issuer.query_owner().unwrap().owner;
    assert_eq!(
        owner_addr,
        Some(Addr::unchecked(owner.address())),
        "owner must be contract instantiate tx signer"
    );
}

#[test]
fn instantiate_with_existing_token_should_set_initial_state_correctly() {
    let app = OsmosisTestApp::new();
    let test_accs = TestEnv::create_default_test_accs(&app, 1);

    let denom = format!("factory/{}/uthb", test_accs[0].address());
    let cw_tokenfactory_issuer = TokenfactoryIssuer::new(
        app,
        &InstantiateMsg::ExistingToken {
            denom: denom.clone(),
        },
        &test_accs[0],
    )
    .unwrap();

    let env = TestEnv {
        cw_tokenfactory_issuer,
        test_accs,
    };

    let owner = &env.test_accs[0];

    let contract_denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    assert_eq!(
        denom, contract_denom,
        "denom stored in contract must be `factory/<contract_addr>/<subdenom>`"
    );

    let is_frozen = env
        .cw_tokenfactory_issuer
        .query_is_frozen()
        .unwrap()
        .is_frozen;
    assert!(!is_frozen, "newly instantiated contract must not be frozen");

    let owner_addr = env.cw_tokenfactory_issuer.query_owner().unwrap().owner;
    assert_eq!(
        owner_addr,
        Some(Addr::unchecked(owner.address())),
        "owner must be contract instantiate tx signer"
    );
}
