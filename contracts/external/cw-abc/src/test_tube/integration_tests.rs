use crate::{
    abc::{
        ClosedConfig, CommonsPhase, CommonsPhaseConfig, CurveType, HatchConfig, MinMax, OpenConfig,
        ReserveToken, SupplyToken,
    },
    msg::{
        CommonsPhaseConfigResponse, CurveInfoResponse, DenomResponse, ExecuteMsg, InstantiateMsg,
        QueryMsg,
    },
    ContractError,
};

use super::test_env::{TestEnv, TestEnvBuilder, DENOM, RESERVE};

use cosmwasm_std::{coins, Decimal, Uint128};
use cw_tokenfactory_issuer::msg::QueryMsg as IssuerQueryMsg;
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest;
use osmosis_test_tube::{
    osmosis_std::types::cosmos::base::v1beta1::Coin, Account, OsmosisTestApp, RunnerError,
};

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
            amount: "1000".to_string(),
        })
    );

    // Query curve
    let curve_info: CurveInfoResponse = abc.query(&QueryMsg::CurveInfo {}).unwrap();
    assert_eq!(
        curve_info,
        CurveInfoResponse {
            reserve: Uint128::new(900),
            supply: Uint128::new(9000),
            funding: Uint128::new(100),
            spot_price: Decimal::percent(10u64),
            reserve_denom: RESERVE.to_string(),
        }
    );

    // Query phase
    let phase: CommonsPhaseConfigResponse = abc.query(&QueryMsg::PhaseConfig {}).unwrap();
    assert_eq!(phase.phase, CommonsPhase::Hatch);
    assert_eq!(
        phase.phase_config,
        CommonsPhaseConfig {
            hatch: HatchConfig {
                contribution_limits: MinMax {
                    min: Uint128::one(),
                    max: Uint128::from(1000000u128),
                },
                initial_raise: MinMax {
                    min: Uint128::one(),
                    max: Uint128::from(1000000u128),
                },
                initial_price: Uint128::one(),
                initial_allocation_ratio: Decimal::percent(10u64),
                exit_tax: Decimal::percent(10u64),
            },
            open: OpenConfig {
                allocation_percentage: Decimal::percent(10u64),
                exit_tax: Decimal::percent(10u64),
            },
            closed: ClosedConfig {},
        }
    );

    // Burn
    abc.execute(
        &ExecuteMsg::Burn {},
        &coins(100, denom.clone()),
        &accounts[0],
    )
    .unwrap();

    // Query curve
    let curve_info: CurveInfoResponse = abc.query(&QueryMsg::CurveInfo {}).unwrap();
    assert_eq!(
        curve_info,
        CurveInfoResponse {
            reserve: Uint128::new(890),
            supply: Uint128::new(8900),
            funding: Uint128::new(110),
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
            amount: "8800".to_string(),
        })
    );
    assert_eq!(
        contract_balance.balance,
        Some(Coin {
            denom: RESERVE.to_string(),
            amount: "990".to_string(),
        })
    );

    // Buy enough tokens to end the hatch phase
    abc.execute(&ExecuteMsg::Buy {}, &coins(1000000, RESERVE), &accounts[0])
        .unwrap();

    // Contract is now in open phase
    let phase: CommonsPhaseConfigResponse = abc.query(&QueryMsg::PhaseConfig {}).unwrap();
    assert_eq!(phase.phase, CommonsPhase::Open);
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
            &coins(1000000000, RESERVE),
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

// TODO
#[test]
fn test_max_supply() {
    // Set a max supply and ensure it does not go over
}
