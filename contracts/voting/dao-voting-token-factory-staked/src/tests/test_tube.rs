use osmosis_test_tube::OsmosisTestApp;

use crate::msg::{DenomResponse, QueryMsg};

use super::test_env::{TestEnv, TestEnvBuilder};

#[test]
fn test_create_new_denom() {
    let app = OsmosisTestApp::new();
    let env_builder = TestEnvBuilder::new();
    let TestEnv { contract, .. } = env_builder.setup(&app);

    let denom: DenomResponse = contract.query(&QueryMsg::Denom {}).unwrap();
    println!("denom: {:?}", denom);
}

// #[test]
// fn test_instantiate_new_denom() {
//     let mut app = mock_app();
//     let issuer_id = app.store_code(issuer_contract());
//     let staking_id = app.store_code(staking_contract());

//     // Populated fields
//     let addr = instantiate_staking(
//         &mut app,
//         staking_id,
//         InstantiateMsg {
//             token_issuer_code_id: issuer_id,
//             owner: Some(Admin::CoreModule {}),
//             manager: Some(ADDR1.to_string()),
//             token_info: TokenInfo::New(NewTokenInfo {
//                 subdenom: DENOM.to_string(),
//                 metadata: Some(crate::msg::NewDenomMetadata {
//                     description: "Awesome token, get it now!".to_string(),
//                     additional_denom_units: Some(vec![DenomUnit {
//                         denom: "njuno".to_string(),
//                         exponent: 9,
//                         aliases: vec![],
//                     }]),
//                     display: DENOM.to_string(),
//                     name: DENOM.to_string(),
//                     symbol: DENOM.to_string(),
//                 }),
//                 initial_balances: vec![InitialBalance {
//                     amount: Uint128::new(100),
//                     address: ADDR1.to_string(),
//                 }],
//                 initial_dao_balance: Some(Uint128::new(900)),
//             }),
//             unstaking_duration: Some(Duration::Height(5)),
//             active_threshold: None,
//         },
//     );

//     let denom = get_denom(&mut app, addr.clone());

//     assert_eq!(denom.denom, format!("factory/{}/{}", addr, DENOM));

//     // Non populated fields
//     instantiate_staking(
//         &mut app,
//         staking_id,
//         InstantiateMsg {
//             token_issuer_code_id: issuer_id,
//             owner: None,
//             manager: None,
//             token_info: TokenInfo::New(NewTokenInfo {
//                 subdenom: DENOM.to_string(),
//                 metadata: Some(crate::msg::NewDenomMetadata {
//                     description: "Awesome token, get it now!".to_string(),
//                     additional_denom_units: Some(vec![DenomUnit {
//                         denom: "njuno".to_string(),
//                         exponent: 9,
//                         aliases: vec![],
//                     }]),
//                     display: DENOM.to_string(),
//                     name: DENOM.to_string(),
//                     symbol: DENOM.to_string(),
//                 }),
//                 initial_balances: vec![InitialBalance {
//                     amount: Uint128::new(100),
//                     address: ADDR1.to_string(),
//                 }],
//                 initial_dao_balance: None,
//             }),
//             unstaking_duration: None,
//             active_threshold: None,
//         },
//     );

//     // No initial balances except DAO.
//     let err = instantiate_staking_error(
//         &mut app,
//         staking_id,
//         InstantiateMsg {
//             token_issuer_code_id: issuer_id,
//             owner: None,
//             manager: None,
//             token_info: TokenInfo::New(NewTokenInfo {
//                 subdenom: DENOM.to_string(),
//                 metadata: Some(crate::msg::NewDenomMetadata {
//                     description: "Awesome token, get it now!".to_string(),
//                     additional_denom_units: Some(vec![DenomUnit {
//                         denom: "njuno".to_string(),
//                         exponent: 9,
//                         aliases: vec![],
//                     }]),
//                     display: DENOM.to_string(),
//                     name: DENOM.to_string(),
//                     symbol: DENOM.to_string(),
//                 }),
//                 initial_balances: vec![],
//                 initial_dao_balance: None,
//             }),
//             unstaking_duration: None,
//             active_threshold: None,
//         },
//     );
//     assert_eq!(err, ContractError::InitialBalancesError {});
// }

// #[test]
// fn test_stake_new_denom() {
//     let mut app = mock_app();
//     let issuer_id = app.store_code(issuer_contract());
//     let staking_id = app.store_code(staking_contract());
//     let addr = instantiate_staking(
//         &mut app,
//         staking_id,
//         InstantiateMsg {
//             token_issuer_code_id: issuer_id,
//             owner: Some(Admin::CoreModule {}),
//             manager: Some(ADDR1.to_string()),
//             token_info: TokenInfo::New(NewTokenInfo {
//                 subdenom: DENOM.to_string(),
//                 metadata: Some(crate::msg::NewDenomMetadata {
//                     description: "Awesome token, get it now!".to_string(),
//                     additional_denom_units: Some(vec![DenomUnit {
//                         denom: "njuno".to_string(),
//                         exponent: 9,
//                         aliases: vec![],
//                     }]),
//                     display: DENOM.to_string(),
//                     name: DENOM.to_string(),
//                     symbol: DENOM.to_string(),
//                     decimals: 6,
//                 }),
//                 initial_balances: vec![InitialBalance {
//                     amount: Uint128::new(100),
//                     address: ADDR1.to_string(),
//                 }],
//                 initial_dao_balance: Some(Uint128::new(900)),
//             }),
//             unstaking_duration: Some(Duration::Height(5)),
//             active_threshold: None,
//         },
//     );

//     // Try and stake a valid denom
//     let denom = get_denom(&mut app, addr.clone()).denom;
//     stake_tokens(&mut app, addr, ADDR1, 100, &denom).unwrap();
//     app.update_block(next_block);
// }
