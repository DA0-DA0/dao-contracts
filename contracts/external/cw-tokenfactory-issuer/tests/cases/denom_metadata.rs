use cw_tokenfactory_issuer::{msg::InstantiateMsg, ContractError};
use osmosis_test_tube::osmosis_std::types::cosmos::bank::v1beta1::{
    DenomUnit, Metadata, QueryDenomMetadataRequest,
};

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn set_denom_metadata_by_contract_owner_should_work() {
    let subdenom = "usthb".to_string();

    // set no metadata
    let env = TestEnv::new(InstantiateMsg::NewToken { subdenom }, 0).unwrap();
    let owner = &env.test_accs[0];

    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let metadata = cw_tokenfactory_issuer::msg::Metadata {
        base: denom.clone(),
        description: "Thai Baht stablecoin".to_string(),
        denom_units: vec![
            cw_tokenfactory_issuer::msg::DenomUnit {
                denom: denom.clone(),
                exponent: 0,
                aliases: vec!["sthb".to_string()],
            },
            cw_tokenfactory_issuer::msg::DenomUnit {
                denom: "sthb".to_string(),
                exponent: 6,
                aliases: vec![],
            },
        ],
        display: "sthb".to_string(),
        name: "Stable Thai Baht".to_string(),
        symbol: "STHB".to_string(),
    };
    env.cw_tokenfactory_issuer
        .set_denom_metadata(metadata, owner)
        .unwrap();

    // // TODO fix me
    // // should set basic metadata
    // assert_eq!(
    //     env.tokenfactory()
    //         .query_denom_metadata(&QueryDenomMetadataRequest {
    //             denom: denom.clone()
    //         })
    //         .unwrap()
    //         .metadata
    //         .unwrap(),
    //     Metadata {
    //         description: "".to_string(),
    //         denom_units: vec![vec![
    //             // must start with `denom` with exponent 0
    //             DenomUnit {
    //                 denom: denom.clone(),
    //                 exponent: 0,
    //                 aliases: vec![],
    //             }
    //         ],]
    //         .concat(),
    //         base: denom.clone(),
    //         display: "".to_string(),
    //         name: "".to_string(),
    //         symbol: "".to_string(),
    //     }
    // );

    // // set denom metadata
    // env.cw_tokenfactory_issuer
    //     .set_denom_metadata(metadata.clone(), owner)
    //     .unwrap();

    // // should update metadata

    // assert_eq!(
    //     env.bank()
    //         .query_denom_metadata(&QueryDenomMetadataRequest {
    //             denom: denom.clone()
    //         })
    //         .unwrap()
    //         .metadata
    //         .unwrap(),
    //     Metadata {
    //         description: metadata.description,
    //         denom_units: metadata
    //             .denom_units
    //             .into_iter()
    //             .map(|d| DenomUnit {
    //                 denom: d.denom,
    //                 exponent: d.exponent,
    //                 aliases: d.aliases,
    //             })
    //             .collect(),
    //         base: denom,
    //         display: metadata.display,
    //         name: metadata.name,
    //         symbol: metadata.symbol,
    //     }
    // );
}

#[test]
fn set_denom_metadata_by_contract_non_owner_should_fail() {
    let subdenom = "usthb".to_string();

    // set no metadata
    let env = TestEnv::new(InstantiateMsg::NewToken { subdenom }, 0).unwrap();
    let non_owner = &env.test_accs[1];

    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let metadata = cw_tokenfactory_issuer::msg::Metadata {
        base: denom.clone(),
        description: "Thai Baht stablecoin".to_string(),
        denom_units: vec![
            cw_tokenfactory_issuer::msg::DenomUnit {
                denom,
                exponent: 0,
                aliases: vec!["sthb".to_string()],
            },
            cw_tokenfactory_issuer::msg::DenomUnit {
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
    let err = env
        .cw_tokenfactory_issuer
        .set_denom_metadata(metadata, non_owner)
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
    let env = TestEnv::new(InstantiateMsg::NewToken { subdenom }, 0).unwrap();
    let owner = &env.test_accs[0];

    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;
    let metadata = cw_tokenfactory_issuer::msg::Metadata {
        base: denom.clone(),
        description: "Thai Baht stablecoin".to_string(),
        denom_units: vec![
            cw_tokenfactory_issuer::msg::DenomUnit {
                denom: denom.clone(),
                exponent: 0,
                aliases: vec!["sthb".to_string()],
            },
            cw_tokenfactory_issuer::msg::DenomUnit {
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
    env.cw_tokenfactory_issuer
        .set_denom_metadata(metadata.clone(), owner)
        .unwrap();

    // should update metadata

    // assert_eq!(
    //     env.bank()
    //         .query_denom_metadata(&QueryDenomMetadataRequest {
    //             denom: denom.clone()
    //         })
    //         .unwrap()
    //         .metadata
    //         .unwrap(),
    //     Metadata {
    //         description: metadata.description,
    //         denom_units: metadata
    //             .denom_units
    //             .into_iter()
    //             .map(|d| DenomUnit {
    //                 denom: d.denom,
    //                 exponent: d.exponent,
    //                 aliases: d.aliases,
    //             })
    //             .collect(),
    //         base: denom,
    //         display: metadata.display,
    //         name: metadata.name,
    //         symbol: metadata.symbol,
    //     }
    // );
}
