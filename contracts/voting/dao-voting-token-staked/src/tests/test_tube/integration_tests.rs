use cosmwasm_std::{Coin, Uint128};
use cw_tokenfactory_issuer::msg::DenomUnit;
use dao_voting::threshold::{ActiveThreshold, ActiveThresholdError};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest;
use osmosis_test_tube::{Account, OsmosisTestApp};

use crate::{
    msg::{ExecuteMsg, InitialBalance, InstantiateMsg, NewDenomMetadata, NewTokenInfo, TokenInfo},
    tests::test_tube::test_env::TfDaoVotingContract,
    ContractError,
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

    let denom = vp_contract.query_denom().unwrap().denom;

    // Stake 100 tokens
    let stake_msg = ExecuteMsg::Stake {};
    vp_contract
        .execute(&stake_msg, &[Coin::new(100, denom)], &accounts[0])
        .unwrap();

    app.increase_time(1);

    // Query voting power
    let voting_power = vp_contract.query_vp(&accounts[0].address(), None).unwrap();
    assert_eq!(voting_power.power, Uint128::new(100));

    // DAO is active (default threshold is absolute count of 75)
    let active = vp_contract.query_active().unwrap().active;
    assert!(active);

    // Unstake 50 tokens
    let unstake_msg = ExecuteMsg::Unstake {
        amount: Uint128::new(50),
    };
    vp_contract
        .execute(&unstake_msg, &[], &accounts[0])
        .unwrap();
    app.increase_time(1);
    let voting_power = vp_contract.query_vp(&accounts[0].address(), None).unwrap();
    assert_eq!(voting_power.power, Uint128::new(50));

    // DAO is not active
    let active = vp_contract.query_active().unwrap().active;
    assert!(!active);

    // Can't claim before unstaking period (2 seconds)
    vp_contract
        .execute(&ExecuteMsg::Claim {}, &[], &accounts[0])
        .unwrap_err();

    // Pass time, unstaking duration is set to 2 seconds
    app.increase_time(5);
    vp_contract
        .execute(&ExecuteMsg::Claim {}, &[], &accounts[0])
        .unwrap();
}

#[test]
fn test_instantiate_no_dao_balance() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();

    let dao = app
        .init_account(&[Coin::new(100000000000, "uosmo")])
        .unwrap();
    let dao_addr = &dao.address();

    let vp_contract = env
        .instantiate(
            &InstantiateMsg {
                token_info: TokenInfo::New(NewTokenInfo {
                    token_issuer_code_id: tf_issuer_id,
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

    let denom = vp_contract.query_denom().unwrap().denom;

    // Check balances
    // Account 0
    let bal = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: env.accounts[0].address(),
            denom: denom.clone(),
        })
        .unwrap();
    assert_eq!(bal.balance.unwrap().amount, Uint128::new(100).to_string());

    // DAO
    let bal = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: dao_addr.to_string(),
            denom,
        })
        .unwrap();
    assert_eq!(bal.balance.unwrap().amount, Uint128::zero().to_string());
}

#[test]
fn test_instantiate_no_metadata() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();

    let dao = app
        .init_account(&[Coin::new(100000000000, "uosmo")])
        .unwrap();

    env.instantiate(
        &InstantiateMsg {
            token_info: TokenInfo::New(NewTokenInfo {
                token_issuer_code_id: tf_issuer_id,
                subdenom: "ucat".to_string(),
                metadata: None,
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
}

#[test]
fn test_instantiate_invalid_metadata_fails() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();

    let dao = app
        .init_account(&[Coin::new(100000000000, "uosmo")])
        .unwrap();

    env.instantiate(
        &InstantiateMsg {
            token_info: TokenInfo::New(NewTokenInfo {
                token_issuer_code_id: tf_issuer_id,
                subdenom: "cat".to_string(),
                metadata: Some(NewDenomMetadata {
                    description: "Awesome token, get it meow!".to_string(),
                    additional_denom_units: Some(vec![DenomUnit {
                        denom: "cat".to_string(),
                        // Exponent 0 is automatically set
                        exponent: 0,
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
    .unwrap_err();
}

#[test]
fn test_instantiate_invalid_active_threshold_count_fails() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();

    let dao = app
        .init_account(&[Coin::new(100000000000, "uosmo")])
        .unwrap();

    let err = env
        .instantiate(
            &InstantiateMsg {
                token_info: TokenInfo::New(NewTokenInfo {
                    token_issuer_code_id: tf_issuer_id,
                    subdenom: "cat".to_string(),
                    metadata: Some(NewDenomMetadata {
                        description: "Awesome token, get it meow!".to_string(),
                        additional_denom_units: Some(vec![DenomUnit {
                            denom: "cat".to_string(),
                            // Exponent 0 is automatically set
                            exponent: 0,
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
                active_threshold: Some(ActiveThreshold::AbsoluteCount {
                    count: Uint128::new(1000),
                }),
            },
            dao,
        )
        .unwrap_err();

    assert_eq!(
        err,
        TfDaoVotingContract::execute_submessage_error(ContractError::ActiveThresholdError(
            ActiveThresholdError::InvalidAbsoluteCount {}
        ))
    );
}

#[test]
fn test_instantiate_no_initial_balances_fails() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new().default_setup(&app);
    let tf_issuer_id = env.get_tf_issuer_code_id();
    let dao = app
        .init_account(&[Coin::new(10000000000000, "uosmo")])
        .unwrap();

    let err = env
        .instantiate(
            &InstantiateMsg {
                token_info: TokenInfo::New(NewTokenInfo {
                    token_issuer_code_id: tf_issuer_id,
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
                    initial_dao_balance: Some(Uint128::new(100000)),
                }),
                unstaking_duration: None,
                active_threshold: None,
            },
            dao,
        )
        .unwrap_err();
    assert_eq!(
        err,
        TfDaoVotingContract::execute_submessage_error(ContractError::InitialBalancesError {})
    );
}
