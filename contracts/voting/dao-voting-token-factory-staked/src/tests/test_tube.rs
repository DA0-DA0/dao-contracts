use cosmwasm_std::{Coin, Uint128};
use cw_tokenfactory_issuer::msg::DenomUnit;
use osmosis_test_tube::{Account, OsmosisTestApp};

use crate::msg::{
    DenomResponse, ExecuteMsg, InitialBalance, InstantiateMsg, NewDenomMetadata, NewTokenInfo,
    QueryMsg, TokenInfo,
};

use super::test_env::{TestEnv, TestEnvBuilder};

#[test]
fn test_stake_unstake_new_denom() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new();
    let TestEnv {
        vp_contract,
        accounts,
        ..
    } = env.default_setup(&app);

    let denom: DenomResponse = vp_contract.query(&QueryMsg::Denom {}).unwrap();

    // Stake 1000 tokens
    let stake_msg = ExecuteMsg::Stake {};
    let stake_result = vp_contract
        .execute(&stake_msg, &[Coin::new(90, denom.denom)], &accounts[0])
        .unwrap();
    println!("stake_result: {:?}", stake_result);
}

#[test]
fn test_instantiate_no_dao_balance() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();
    let dao = app
        .init_account(&[Coin::new(100000000000, "uosmo")])
        .unwrap();

    let _vp_contract = env
        .instantiate(
            &InstantiateMsg {
                token_issuer_code_id: tf_issuer_id,
                owner: None,
                manager: None,
                token_info: TokenInfo::New(NewTokenInfo {
                    subdenom: "ucat".to_string(),
                    metadata: Some(NewDenomMetadata {
                        description: "Awesome token, get it meow!".to_string(),
                        additional_denom_units: Some(vec![DenomUnit {
                            denom: "cat".to_string(),
                            exponent: 6,
                            aliases: vec![],
                        }]),
                        display: "cat".to_string(),
                        name: "Cat Token".to_string(),
                        symbol: "CAT".to_string(),
                    }),
                    initial_balances: vec![InitialBalance {
                        amount: Uint128::new(100),
                        address: env.accounts[0].address(),
                    }],
                    initial_dao_balance: None,
                }),
                unstaking_duration: None,
                active_threshold: None,
            },
            dao,
        )
        .unwrap();

    // TODO check balances
}

#[test]
fn test_instantiate_no_initial_balances_fails() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();
    let dao = app.init_account(&[Coin::new(100000000, "uosmo")]).unwrap();

    let _err = env
        .instantiate(
            &InstantiateMsg {
                token_issuer_code_id: tf_issuer_id,
                owner: None,
                manager: None,
                token_info: TokenInfo::New(NewTokenInfo {
                    subdenom: "ucat".to_string(),
                    metadata: Some(NewDenomMetadata {
                        description: "Awesome token, get it meow!".to_string(),
                        additional_denom_units: Some(vec![DenomUnit {
                            denom: "cat".to_string(),
                            exponent: 6,
                            aliases: vec![],
                        }]),
                        display: "cat".to_string(),
                        name: "Cat Token".to_string(),
                        symbol: "CAT".to_string(),
                    }),
                    initial_balances: vec![],
                    initial_dao_balance: None,
                }),
                unstaking_duration: None,
                active_threshold: None,
            },
            dao,
        )
        .unwrap_err();

    // TODO check error
    // assert_eq!(err, ContractError::InitialBalancesError {});
}

// TODO test invalid metatdata fails
// TODO test active threshold works as intended
// TODO stretch goal full dao integration test
