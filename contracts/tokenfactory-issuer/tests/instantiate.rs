use std::vec;

use cosmwasm_std::coins;

use osmosis_testing::{
    cosmrs::proto::cosmos::bank::v1beta1::{DenomUnit, Metadata, QueryDenomMetadataRequest},
    Account, OsmosisTestApp, RunnerError,
};
use tokenfactory_issuer::msg::InstantiateMsg;

mod helpers;

use helpers::{TestEnv, TokenfactoryIssuer};

// new denom

#[test]
fn instantiate_with_new_token_shoud_set_initial_state_correctly() {
    let subdenom = "uthb".to_string();
    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom: subdenom.clone(),
            metadata: None,
        },
        0,
    )
    .unwrap();

    let owner = &env.test_accs[0];

    // check tokenfactory's token admin
    let denom = format!(
        "factory/{}/{}",
        env.tokenfactory_issuer.contract_addr, subdenom
    );

    assert_eq!(
        env.token_admin(&denom),
        env.tokenfactory_issuer.contract_addr,
        "token admin must be tokenfactory-issuer contract"
    );

    // check initial contract state
    let contract_denom = env.tokenfactory_issuer.query_denom().unwrap().denom;
    assert_eq!(
        denom, contract_denom,
        "denom stored in contract must be `factory/<contract_addr>/<subdenom>`"
    );

    let is_frozen = env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen;
    assert!(!is_frozen, "newly instantiated contract must not be frozen");

    let owner_addr = env.tokenfactory_issuer.query_owner().unwrap().address;
    assert_eq!(
        owner_addr,
        owner.address(),
        "owner must be contract instantiate tx signer"
    );
}

#[test]
fn instantiate_with_new_token_shoud_set_hook_correctly() {
    let subdenom = "uthb".to_string();
    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom: subdenom.clone(),
            metadata: None,
        },
        0,
    )
    .unwrap();

    let owner = &env.test_accs[0];

    let denom = format!(
        "factory/{}/{}",
        env.tokenfactory_issuer.contract_addr, subdenom
    );

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

    assert_eq!(err, RunnerError::ExecuteError { msg: format!("failed to execute message; message index: 0: failed to call before send hook for denom {denom}: The contract is frozen for denom \"{denom}\": execute wasm contract failed") });
}

#[test]
fn instantiate_with_denom_metadata() {
    let subdenom = "usthb".to_string();
    let additional_metadata = tokenfactory_issuer::msg::AdditionalMetadata {
        description: "Thai Baht stablecoin".to_string(),
        denom_units: vec![tokenfactory_issuer::msg::DenomUnit {
            denom: "sthb".to_string(),
            exponent: 6,
            aliases: vec![],
        }],
        display: "sthb".to_string(),
        name: "Stable Thai Baht".to_string(),
        symbol: "STHB".to_string(),
    };

    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom: subdenom.clone(),
            metadata: Some(additional_metadata.clone()),
        },
        0,
    )
    .unwrap();

    let denom = format!(
        "factory/{}/{}",
        env.tokenfactory_issuer.contract_addr, subdenom
    );

    assert_eq!(
        env.bank()
            .query_denom_metadata(&QueryDenomMetadataRequest {
                denom: denom.clone()
            })
            .unwrap()
            .metadata
            .unwrap(),
        Metadata {
            description: additional_metadata.description,
            denom_units: vec![
                vec![
                    // must start with `denom` with exponent 0
                    DenomUnit {
                        denom: denom.clone(),
                        exponent: 0,
                        aliases: vec![],
                    }
                ],
                additional_metadata
                    .denom_units
                    .into_iter()
                    .map(|d| DenomUnit {
                        denom: d.denom,
                        exponent: d.exponent,
                        aliases: d.aliases,
                    })
                    .collect()
            ]
            .concat(),
            base: denom,
            display: additional_metadata.display,
            name: additional_metadata.name,
            symbol: additional_metadata.symbol,
        }
    );
}

// existing denom

#[test]
fn instantiate_with_existing_token_should_set_initial_state_correctly() {
    let app = OsmosisTestApp::new();
    let test_accs = TestEnv::create_default_test_accs(&app, 1);

    let denom = format!("factory/{}/uthb", test_accs[0].address());
    let tokenfactory_issuer = TokenfactoryIssuer::new(
        app,
        &InstantiateMsg::ExistingToken {
            denom: denom.clone(),
        },
        &test_accs[0],
    )
    .unwrap();

    let env = TestEnv {
        tokenfactory_issuer,
        test_accs,
    };

    let owner = &env.test_accs[0];

    let contract_denom = env.tokenfactory_issuer.query_denom().unwrap().denom;
    assert_eq!(
        denom, contract_denom,
        "denom stored in contract must be `factory/<contract_addr>/<subdenom>`"
    );

    let is_frozen = env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen;
    assert!(!is_frozen, "newly instantiated contract must not be frozen");

    let owner_addr = env.tokenfactory_issuer.query_owner().unwrap().address;
    assert_eq!(
        owner_addr,
        owner.address(),
        "owner must be contract instantiate tx signer"
    );
}
