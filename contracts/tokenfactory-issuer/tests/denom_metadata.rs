mod helpers;
use helpers::{TestEnv, TokenfactoryIssuer};
use osmosis_testing::cosmrs::proto::cosmos::bank::v1beta1::{
    DenomUnit, Metadata, QueryDenomMetadataRequest,
};
use tokenfactory_issuer::{msg::InstantiateMsg, ContractError};

#[test]
fn instantiate_with_denom_metadata_should_set_denom_metadata() {
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

#[test]
fn set_denom_metadata_by_contract_owner_should_work() {
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

    // set no metadata
    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom,
            metadata: None,
        },
        0,
    )
    .unwrap();
    let owner = &env.test_accs[0];

    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    // should set basic metadata
    assert_eq!(
        env.bank()
            .query_denom_metadata(&QueryDenomMetadataRequest {
                denom: denom.clone()
            })
            .unwrap()
            .metadata
            .unwrap(),
        Metadata {
            description: "".to_string(),
            denom_units: vec![vec![
                // must start with `denom` with exponent 0
                DenomUnit {
                    denom: denom.clone(),
                    exponent: 0,
                    aliases: vec![],
                }
            ],]
            .concat(),
            base: denom.clone(),
            display: "".to_string(),
            name: "".to_string(),
            symbol: "".to_string(),
        }
    );

    // set denom metadata
    env.tokenfactory_issuer
        .set_denom_metadata(additional_metadata.clone(), owner)
        .unwrap();

    // should update metadata

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

#[test]
fn set_denom_metadata_by_contract_non_owner_should_fail() {
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

    // set no metadata
    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom,
            metadata: None,
        },
        0,
    )
    .unwrap();
    let non_owner = &env.test_accs[1];

    // set denom metadata
    let err = env
        .tokenfactory_issuer
        .set_denom_metadata(additional_metadata, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    )
}

#[test]
fn set_denom_metadata_with_base_denom_unit_should_overides_default_base_denom_unit() {
    let subdenom = "usthb".to_string();

    // set no metadata
    let env = TestEnv::new(
        InstantiateMsg::NewToken {
            subdenom,
            metadata: None,
        },
        0,
    )
    .unwrap();
    let owner = &env.test_accs[0];

    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    let additional_metadata = tokenfactory_issuer::msg::AdditionalMetadata {
        description: "Thai Baht stablecoin".to_string(),
        denom_units: vec![
            tokenfactory_issuer::msg::DenomUnit {
                denom: denom.clone(),
                exponent: 0,
                aliases: vec!["sthb".to_string()],
            },
            tokenfactory_issuer::msg::DenomUnit {
                denom: "sthb".to_string(),
                exponent: 6,
                aliases: vec![],
            },
        ],
        display: "sthb".to_string(),
        name: "Stable Thai Baht".to_string(),
        symbol: "STHB".to_string(),
    };

    // set denom metadata
    env.tokenfactory_issuer
        .set_denom_metadata(additional_metadata.clone(), owner)
        .unwrap();

    // should update metadata

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
            denom_units: additional_metadata
                .denom_units
                .into_iter()
                .map(|d| DenomUnit {
                    denom: d.denom,
                    exponent: d.exponent,
                    aliases: d.aliases,
                })
                .collect(),
            base: denom,
            display: additional_metadata.display,
            name: additional_metadata.name,
            symbol: additional_metadata.symbol,
        }
    );
}
