use crate::{
    abc::{
        ClosedConfig, CommonsPhase, CommonsPhaseConfig, CurveType, HatchConfig, MinMax, OpenConfig,
        ReserveToken, SupplyToken,
    },
    msg::{
        CommonsPhaseConfigResponse, CurveInfoResponse, DenomResponse, ExecuteMsg,
        HatcherAllowlistConfigMsg, HatcherAllowlistEntryMsg, InstantiateMsg, QueryMsg,
        QuoteResponse,
    },
    state::HatcherAllowlistConfigType,
    ContractError,
};

use super::test_env::{TestEnv, TestEnvBuilder, DENOM, RESERVE};

use cosmwasm_std::{coins, Decimal, Uint128, Uint64};
use cw_tokenfactory_issuer::msg::QueryMsg as IssuerQueryMsg;
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest;
use osmosis_test_tube::{osmosis_std::types::cosmos::base::v1beta1::Coin, Account, OsmosisTestApp};

#[test]
fn test_happy_path() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ref tf_issuer,
        ..
    } = env;

    // Query buy quote
    let quote = abc
        .query::<QuoteResponse>(&QueryMsg::BuyQuote {
            payment: Uint128::new(1000u128),
        })
        .unwrap();
    assert_eq!(
        quote,
        QuoteResponse {
            new_reserve: Uint128::new(900u128),
            funded: Uint128::new(100u128),
            amount: Uint128::new(9000u128),
            new_supply: Uint128::new(9000u128),
        }
    );

    // Buy tokens
    abc.execute(&ExecuteMsg::Buy {}, &coins(1000, RESERVE), &accounts[0])
        .unwrap();

    // Query denom
    let denom = tf_issuer
        .query::<DenomResponse>(&IssuerQueryMsg::Denom {})
        .unwrap()
        .denom;

    // Query balances
    let user_balance = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: accounts[0].address(),
            denom: denom.clone(),
        })
        .unwrap();
    let contract_balance = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: abc.contract_addr.to_string(),
            denom: RESERVE.to_string(),
        })
        .unwrap();

    // Check balances
    assert_eq!(
        user_balance.balance,
        Some(Coin {
            denom: denom.clone(),
            amount: "9000".to_string(),
        })
    );
    assert_eq!(
        contract_balance.balance,
        Some(Coin {
            denom: RESERVE.to_string(),
            amount: "900".to_string(), // Minus 10% to fees_recipient
        })
    );

    // Query curve
    let curve_info: CurveInfoResponse = abc.query(&QueryMsg::CurveInfo {}).unwrap();
    assert_eq!(
        curve_info,
        CurveInfoResponse {
            reserve: Uint128::new(900),
            supply: Uint128::new(9000),
            funding: Uint128::new(0),
            spot_price: Decimal::percent(10u64),
            reserve_denom: RESERVE.to_string(),
        }
    );

    // Query phase
    let phase: CommonsPhaseConfigResponse = abc.query(&QueryMsg::PhaseConfig {}).unwrap();
    assert!(matches!(phase.phase, CommonsPhase::Hatch));
    assert_eq!(
        phase.phase_config,
        CommonsPhaseConfig {
            hatch: HatchConfig {
                contribution_limits: MinMax {
                    min: Uint128::from(10u128),
                    max: Uint128::from(1000000u128),
                },
                initial_raise: MinMax {
                    min: Uint128::from(10u128),
                    max: Uint128::from(900_000u128),
                },
                entry_fee: Decimal::percent(10u64),
            },
            open: OpenConfig {
                entry_fee: Decimal::percent(10u64),
                exit_fee: Decimal::percent(10u64),
            },
            closed: ClosedConfig {},
        }
    );

    // Trying to sell is an error
    let err = abc
        .execute(
            &ExecuteMsg::Sell {},
            &coins(1000, denom.clone()),
            &accounts[0],
        )
        .unwrap_err();
    assert_eq!(err, abc.execute_error(ContractError::CommonsHatch {}));

    // Buy enough tokens to end the hatch phase
    abc.execute(&ExecuteMsg::Buy {}, &coins(999999, RESERVE), &accounts[1])
        .unwrap();

    // Contract is now in open phase
    let phase: CommonsPhaseConfigResponse = abc.query(&QueryMsg::PhaseConfig {}).unwrap();
    assert_eq!(phase.phase, CommonsPhase::Open);

    // Query sell quote
    let quote = abc
        .query::<QuoteResponse>(&QueryMsg::SellQuote {
            payment: Uint128::new(1000u128),
        })
        .unwrap();
    assert_eq!(
        quote,
        QuoteResponse {
            new_reserve: Uint128::new(900800u128),
            funded: Uint128::new(10u128),
            amount: Uint128::new(90u128),
            new_supply: Uint128::new(9008000u128),
        }
    );

    // Sell
    abc.execute(
        &ExecuteMsg::Sell {},
        &coins(1000, denom.clone()),
        &accounts[0],
    )
    .unwrap();

    // Query curve
    let curve_info: CurveInfoResponse = abc.query(&QueryMsg::CurveInfo {}).unwrap();
    assert_eq!(
        curve_info,
        CurveInfoResponse {
            reserve: Uint128::new(900800u128),
            supply: Uint128::new(9008000u128),
            funding: Uint128::new(0),
            spot_price: Decimal::percent(10u64),
            reserve_denom: RESERVE.to_string(),
        }
    );

    // Query balances
    let user_balance = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: accounts[0].address(),
            denom: denom.clone(),
        })
        .unwrap();
    let contract_balance = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: abc.contract_addr.to_string(),
            denom: RESERVE.to_string(),
        })
        .unwrap();

    // Check balances
    assert_eq!(
        user_balance.balance,
        Some(Coin {
            denom: denom.clone(),
            amount: "8000".to_string(),
        })
    );
    assert_eq!(
        contract_balance.balance,
        Some(Coin {
            denom: RESERVE.to_string(),
            amount: "900800".to_string(),
        })
    );
}

#[test]
fn test_contribution_limits_enforced() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ..
    } = env;

    // Buy more tokens then the max contribution limit errors
    let err = abc
        .execute(
            &ExecuteMsg::Buy {},
            &coins(1_000_000_000, RESERVE),
            &accounts[0],
        )
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::ContributionLimit {
            min: Uint128::from(10u128),
            max: Uint128::from(1000000u128),
        })
    );

    // Buy less tokens then the min contribution limit errors
    let err = abc
        .execute(&ExecuteMsg::Buy {}, &coins(1, RESERVE), &accounts[0])
        .unwrap_err();

    assert_eq!(
        err,
        abc.execute_error(ContractError::ContributionLimit {
            min: Uint128::from(10u128),
            max: Uint128::from(1000000u128),
        })
    );
}

#[test]
fn test_max_supply() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ..
    } = env;

    // Buy enough tokens to end the hatch phase
    abc.execute(
        &ExecuteMsg::Buy {},
        &coins(1_000_000, RESERVE),
        &accounts[0],
    )
    .unwrap();

    // Buy enough tokens to trigger a max supply error
    let err = abc
        .execute(
            &ExecuteMsg::Buy {},
            &coins(10000000000000, RESERVE),
            &accounts[0],
        )
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::CannotExceedMaxSupply {
            max: Uint128::from(1000000000u128)
        })
    );

    // Only owner can update the max supply
    let err = abc
        .execute(
            &ExecuteMsg::UpdateMaxSupply { max_supply: None },
            &[],
            &accounts[1],
        )
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );

    // Update the max supply to no limit
    abc.execute(
        &ExecuteMsg::UpdateMaxSupply { max_supply: None },
        &[],
        &accounts[0],
    )
    .unwrap();

    // Purchase large amount of coins succeeds
    abc.execute(
        &ExecuteMsg::Buy {},
        &coins(10000000000000, RESERVE),
        &accounts[0],
    )
    .unwrap();
}

#[test]
fn test_allowlist() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let instantiate_msg = InstantiateMsg {
        token_issuer_code_id: 0,
        funding_pool_forwarding: Some("replaced to accounts[0]".to_string()),
        supply: SupplyToken {
            subdenom: DENOM.to_string(),
            metadata: None,
            decimals: 6,
            max_supply: Some(Uint128::from(1000000000u128)),
        },
        reserve: ReserveToken {
            denom: RESERVE.to_string(),
            decimals: 6,
        },
        phase_config: CommonsPhaseConfig {
            hatch: HatchConfig {
                contribution_limits: MinMax {
                    min: Uint128::from(10u128),
                    max: Uint128::from(1000000u128),
                },
                initial_raise: MinMax {
                    min: Uint128::from(10u128),
                    max: Uint128::from(1000000u128),
                },
                entry_fee: Decimal::percent(10u64),
            },
            open: OpenConfig {
                entry_fee: Decimal::percent(10u64),
                exit_fee: Decimal::percent(10u64),
            },
            closed: ClosedConfig {},
        },
        hatcher_allowlist: Some(vec![HatcherAllowlistEntryMsg {
            addr: "replaced to accounts[9]".to_string(),
            config: HatcherAllowlistConfigMsg {
                config_type: HatcherAllowlistConfigType::Address {},
                contribution_limits_override: None,
            },
        }]),
        curve_type: CurveType::Constant {
            value: Uint128::one(),
            scale: 1,
        },
    };
    let env = builder.setup(&app, instantiate_msg).unwrap();
    let TestEnv {
        ref abc,
        ref accounts,
        ..
    } = env;

    // Only owner can update hatch list
    let err = abc
        .execute(
            &ExecuteMsg::UpdateHatchAllowlist {
                to_add: vec![
                    HatcherAllowlistEntryMsg {
                        addr: accounts[0].address(),
                        config: HatcherAllowlistConfigMsg {
                            config_type: HatcherAllowlistConfigType::Address {},
                            contribution_limits_override: None,
                        },
                    },
                    HatcherAllowlistEntryMsg {
                        addr: accounts[1].address(),
                        config: HatcherAllowlistConfigMsg {
                            config_type: HatcherAllowlistConfigType::Address {},
                            contribution_limits_override: None,
                        },
                    },
                ],
                to_remove: vec![],
            },
            &[],
            &accounts[1],
        )
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );

    // Update the allowlist
    abc.execute(
        &ExecuteMsg::UpdateHatchAllowlist {
            to_add: vec![
                HatcherAllowlistEntryMsg {
                    addr: accounts[0].address(),
                    config: HatcherAllowlistConfigMsg {
                        config_type: HatcherAllowlistConfigType::Address {},
                        contribution_limits_override: None,
                    },
                },
                HatcherAllowlistEntryMsg {
                    addr: accounts[1].address(),
                    config: HatcherAllowlistConfigMsg {
                        config_type: HatcherAllowlistConfigType::Address {},
                        contribution_limits_override: None,
                    },
                },
            ],
            to_remove: vec![],
        },
        &[],
        &accounts[0],
    )
    .unwrap();

    // Account not on the hatch allowlist can't purchase
    let err = abc
        .execute(&ExecuteMsg::Buy {}, &coins(1000, RESERVE), &accounts[3])
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::SenderNotAllowlisted {
            sender: accounts[3].address()
        })
    );

    // Account on allowlist can purchase
    abc.execute(&ExecuteMsg::Buy {}, &coins(1000, RESERVE), &accounts[1])
        .unwrap();
}

#[test]
fn test_close_curve() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ref tf_issuer,
        ..
    } = env;

    // Query denom
    let denom = tf_issuer
        .query::<DenomResponse>(&IssuerQueryMsg::Denom {})
        .unwrap()
        .denom;

    // Buy enough tokens to end the hatch phase
    abc.execute(&ExecuteMsg::Buy {}, &coins(1000000, RESERVE), &accounts[0])
        .unwrap();

    // Only owner can close the curve
    let err = abc
        .execute(&ExecuteMsg::Close {}, &[], &accounts[1])
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );

    // Owner closes curve
    abc.execute(&ExecuteMsg::Close {}, &[], &accounts[0])
        .unwrap();

    // Can no longer buy
    let err = abc
        .execute(&ExecuteMsg::Buy {}, &coins(1000, RESERVE), &accounts[0])
        .unwrap_err();
    assert_eq!(err, abc.execute_error(ContractError::CommonsClosed {}));

    // Can sell
    abc.execute(&ExecuteMsg::Sell {}, &coins(100, denom), &accounts[0])
        .unwrap();
}

// TODO maybe we don't allow for updating the curve in the MVP as it could lead
// to weird edge cases?
#[test]
fn test_update_curve() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ref tf_issuer,
        ..
    } = env;

    // Query denom
    let denom = tf_issuer
        .query::<DenomResponse>(&IssuerQueryMsg::Denom {})
        .unwrap()
        .denom;

    // Buy enough tokens to end the hatch phase
    abc.execute(
        &ExecuteMsg::Buy {},
        &coins(1_000_000, RESERVE),
        &accounts[0],
    )
    .unwrap();

    // Only owner can update the curve
    let err = abc
        .execute(
            &ExecuteMsg::UpdateCurve {
                curve_type: CurveType::Linear {
                    slope: Uint128::new(2),
                    scale: 5,
                },
            },
            &[],
            &accounts[1],
        )
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );

    // Owner updates curve
    abc.execute(
        &ExecuteMsg::UpdateCurve {
            curve_type: CurveType::Linear {
                slope: Uint128::new(2),
                scale: 5,
            },
        },
        &[],
        &accounts[0],
    )
    .unwrap();

    // All tokens are sold successfully
    let user_balance = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: accounts[0].address(),
            denom: denom.clone(),
        })
        .unwrap();
    assert_eq!(
        user_balance.balance,
        Some(Coin {
            denom: denom.clone(),
            amount: "9000000".to_string(),
        })
    );

    abc.execute(
        &ExecuteMsg::Sell {},
        &coins(9000000, denom.clone()),
        &accounts[0],
    )
    .unwrap();

    // No money is left over in the contract
    let contract_balance = env
        .bank()
        .query_balance(&QueryBalanceRequest {
            address: abc.contract_addr.to_string(),
            denom: RESERVE.to_string(),
        })
        .unwrap();
    assert_eq!(
        contract_balance.balance,
        Some(Coin {
            denom: RESERVE.to_string(),
            amount: "0".to_string(),
        })
    );
}

#[test]
fn test_dao_hatcher() {
    let app = OsmosisTestApp::new();
    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ..
    } = env;

    // Setup a dao with the 1st half of accounts
    let dao_ids = env.init_dao_ids();
    let daos: Vec<_> = (0..5)
        .into_iter()
        .map(|_| env.setup_default_dao(dao_ids))
        .collect();
    app.increase_time(1u64);

    // Update hatcher allowlist for DAO membership
    // The max contribution of 50 should have the highest priority
    for (i, dao) in daos.iter().enumerate() {
        let result = abc.execute(
            &ExecuteMsg::UpdateHatchAllowlist {
                to_add: vec![HatcherAllowlistEntryMsg {
                    addr: dao.contract_addr.to_string(),
                    config: HatcherAllowlistConfigMsg {
                        config_type: HatcherAllowlistConfigType::DAO {
                            priority: Some(Uint64::MAX - Uint64::new(i as u64)), // Insert in reverse priority to ensure insertion ordering is valid
                        },
                        contribution_limits_override: Some(MinMax {
                            min: Uint128::one(),
                            max: Uint128::from(10u128) * Uint128::from(i as u128 + 1u128),
                        }),
                    },
                }],
                to_remove: vec![],
            },
            &[],
            &accounts[0],
        );
        assert!(result.is_ok());
    }

    // Let's also insert a dao with no priority to make sure it's added to the end
    let dao = env.setup_default_dao(dao_ids);
    let result = abc.execute(
        &ExecuteMsg::UpdateHatchAllowlist {
            to_add: vec![HatcherAllowlistEntryMsg {
                addr: dao.contract_addr.to_string(),
                config: HatcherAllowlistConfigMsg {
                    config_type: HatcherAllowlistConfigType::DAO { priority: None },
                    contribution_limits_override: Some(MinMax {
                        min: Uint128::one(),
                        max: Uint128::from(100u128),
                    }),
                },
            }],
            to_remove: vec![],
        },
        &[],
        &accounts[0],
    );
    assert!(result.is_ok());

    // Also add a DAO tied for the highest priority
    // This should not update contribution limit, because the 1st DAO was added first and user is a member of it
    let dao = env.setup_default_dao(dao_ids);
    let result = abc.execute(
        &ExecuteMsg::UpdateHatchAllowlist {
            to_add: vec![HatcherAllowlistEntryMsg {
                addr: dao.contract_addr.to_string(),
                config: HatcherAllowlistConfigMsg {
                    config_type: HatcherAllowlistConfigType::DAO {
                        priority: Some(Uint64::MAX - Uint64::from(4u64)),
                    },
                    contribution_limits_override: Some(MinMax {
                        min: Uint128::one(),
                        max: Uint128::from(1000u128),
                    }),
                },
            }],
            to_remove: vec![],
        },
        &[],
        &accounts[0],
    );
    assert!(result.is_ok());

    // Check contribution limit at this point
    let err = abc
        .execute(&ExecuteMsg::Buy {}, &coins(1000, RESERVE), &accounts[0])
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::ContributionLimit {
            min: Uint128::one(),
            max: Uint128::from(50u128)
        })
    );

    // Check removing a dao config updates the contribution limit
    let result = abc.execute(
        &ExecuteMsg::UpdateHatchAllowlist {
            to_add: vec![],
            to_remove: vec![daos.last().unwrap().contract_addr.to_string()],
        },
        &[],
        &accounts[0],
    );
    assert!(result.is_ok());

    // The error should say 1k is the max contribution now
    let err = abc
        .execute(&ExecuteMsg::Buy {}, &coins(2000, RESERVE), &accounts[0])
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::ContributionLimit {
            min: Uint128::one(),
            max: Uint128::from(1000u128)
        })
    );

    // Adhering to the limit makes this ok now
    let result = abc.execute(&ExecuteMsg::Buy {}, &coins(40, RESERVE), &accounts[0]);
    assert!(result.is_ok());

    // Check not allowlisted
    let result = abc.execute(
        &ExecuteMsg::Buy {},
        &coins(1000, RESERVE),
        &accounts[accounts.len() - 1],
    );
    assert_eq!(
        result.unwrap_err(),
        abc.execute_error(ContractError::SenderNotAllowlisted {
            sender: accounts[accounts.len() - 1].address().to_string()
        })
    );

    // Check an address config takes complete priority
    let result = abc.execute(
        &ExecuteMsg::UpdateHatchAllowlist {
            to_add: vec![HatcherAllowlistEntryMsg {
                addr: accounts[0].address(),
                config: HatcherAllowlistConfigMsg {
                    config_type: HatcherAllowlistConfigType::Address {},
                    contribution_limits_override: Some(MinMax {
                        min: Uint128::one(),
                        max: Uint128::from(2000u128),
                    }),
                },
            }],
            to_remove: vec![],
        },
        &[],
        &accounts[0],
    );
    assert!(result.is_ok());

    // The user has already funded 40, so providing their limit should error
    let err = abc
        .execute(&ExecuteMsg::Buy {}, &coins(2000, RESERVE), &accounts[0])
        .unwrap_err();
    assert_eq!(
        err,
        abc.execute_error(ContractError::ContributionLimit {
            min: Uint128::one(),
            max: Uint128::from(2000u128)
        })
    );

    // Funding the remainder is ok
    let result = abc.execute(&ExecuteMsg::Buy {}, &coins(1960, RESERVE), &accounts[0]);
    assert!(result.is_ok());
}
