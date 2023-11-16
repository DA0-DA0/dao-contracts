use cosmwasm_std::{
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    Decimal, DepsMut, OwnedDeps, Uint128,
};
use dao_interface::token::NewDenomMetadata;
use std::marker::PhantomData;
use token_bindings::TokenFactoryQuery;

use crate::abc::{
    ClosedConfig, CommonsPhaseConfig, CurveType, HatchConfig, MinMax, OpenConfig, ReserveToken,
    SupplyToken,
};
use crate::contract;
use crate::contract::CwAbcResult;
use crate::msg::InstantiateMsg;

pub(crate) mod prelude {
    pub use super::{
        default_instantiate_msg, default_supply_metadata, mock_tf_dependencies, TEST_CREATOR,
        TEST_RESERVE_DENOM, TEST_SUPPLY_DENOM, _TEST_BUYER, _TEST_INVESTOR,
    };
    pub use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    pub use speculoos::prelude::*;
}

pub const TEST_RESERVE_DENOM: &str = "satoshi";
pub const TEST_CREATOR: &str = "creator";
pub const _TEST_INVESTOR: &str = "investor";
pub const _TEST_BUYER: &str = "buyer";

pub const TEST_SUPPLY_DENOM: &str = "subdenom";

pub fn default_supply_metadata() -> NewDenomMetadata {
    NewDenomMetadata {
        name: "Bonded".to_string(),
        symbol: "EPOXY".to_string(),
        description: "Forever".to_string(),
        display: "EPOXY".to_string(),
        additional_denom_units: None,
    }
}

pub fn default_instantiate_msg(
    decimals: u8,
    reserve_decimals: u8,
    curve_type: CurveType,
) -> InstantiateMsg {
    InstantiateMsg {
        token_issuer_code_id: 1,
        supply: SupplyToken {
            subdenom: TEST_SUPPLY_DENOM.to_string(),
            metadata: Some(default_supply_metadata()),
            decimals,
        },
        reserve: ReserveToken {
            denom: TEST_RESERVE_DENOM.to_string(),
            decimals: reserve_decimals,
        },
        phase_config: CommonsPhaseConfig {
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
                exit_tax: Decimal::zero(),
            },
            open: OpenConfig {
                allocation_percentage: Decimal::percent(10u64),
                exit_tax: Decimal::percent(10u64),
            },
            closed: ClosedConfig {},
        },
        hatcher_allowlist: None,
        curve_type,
    }
}

pub fn mock_init(deps: DepsMut<TokenFactoryQuery>, init_msg: InstantiateMsg) -> CwAbcResult {
    let info = mock_info(TEST_CREATOR, &[]);
    let env = mock_env();
    contract::instantiate(deps, env, info, init_msg)
}

pub fn mock_tf_dependencies(
) -> OwnedDeps<MockStorage, MockApi, MockQuerier<TokenFactoryQuery>, TokenFactoryQuery> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::<TokenFactoryQuery>::new(&[]),
        custom_query_type: PhantomData::<TokenFactoryQuery>,
    }
}

// fn setup_test(
//     deps: DepsMut<TokenFactoryMsg>,
//     decimals: u8,
//     reserve_decimals: u8,
//     curve_type: CurveType,
// ) {
//     // this matches `linear_curve` test case from curves.rs
//     let creator = String::from(CREATOR);
//     let msg = default_instantiate_msg(decimals, reserve_decimals, curve_type);
//     let info = mock_info(&creator, &[]);

//     // make sure we can instantiate with this
//     let res = instantiate(deps, mock_env(), info, msg).unwrap();
//     assert_eq!(0, res.messages.len());
// }

// // Mock token factory querier dependencies
// #[test]
// fn proper_instantiation() -> CwAbcResult<()> {
//     let mut deps = mock_tf_dependencies();

//     // this matches `linear_curve` test case from curves.rs
//     let creator = String::from("creator");
//     let curve_type = CurveType::SquareRoot {
//         slope: Uint128::new(1),
//         scale: 1,
//     };
//     let msg = default_instantiate_msg(2, 8, curve_type.clone());
//     let info = mock_info(&creator, &[]);

//     // make sure we can instantiate with this
//     let res = instantiate(deps.as_mut(), mock_env(), info, msg)?;
//     assert_that!(res.messages.len()).is_equal_to(1);
//     let submsg = res.messages.get(0).unwrap();
//     assert_that!(submsg.msg).is_equal_to(CosmosMsg::Custom(WasmMsg::Execute {
//         contract_addr: (),
//         msg: (),
//         funds: (),
//     }));

//     // TODO!
//     // // token info is proper
//     // let token = query_token_info(deps.as_ref()).unwrap();
//     // assert_that!(&token.name, &msg.name);
//     // assert_that!(&token.symbol, &msg.symbol);
//     // assert_that!(token.decimals, 2);
//     // assert_that!(token.total_supply, Uint128::zero());

//     // curve state is sensible
//     let state = query_curve_info(deps.as_ref(), curve_type.to_curve_fn())?;
//     assert_that!(state.reserve).is_equal_to(Uint128::zero());
//     assert_that!(state.supply).is_equal_to(Uint128::zero());
//     assert_that!(state.reserve_denom.as_str()).is_equal_to(TEST_RESERVE_DENOM);
//     // spot price 0 as supply is 0
//     assert_that!(state.spot_price).is_equal_to(Decimal::zero());

//     // curve type is stored properly
//     let curve = CURVE_TYPE.load(&deps.storage).unwrap();
//     assert_eq!(curve_type, curve);

//     // no balance
//     // assert_eq!(get_balance(deps.as_ref(), &creator), Uint128::zero());

//     Ok(())
// }

// #[test]
// fn buy_issues_tokens() {
//     let mut deps = mock_dependencies();
//     let curve_type = CurveType::Linear {
//         slope: Uint128::new(1),
//         scale: 1,
//     };
//     setup_test(deps.as_mut(), 2, 8, curve_type.clone());

//     // succeeds with proper token (5 BTC = 5*10^8 satoshi)
//     let info = mock_info(INVESTOR, &coins(500_000_000, DENOM));
//     let buy = ExecuteMsg::Buy {};
//     execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

//     // bob got 1000 EPOXY (10.00)
//     assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
//     assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::zero());

//     // send them all to buyer
//     let info = mock_info(INVESTOR, &[]);
//     let send = ExecuteMsg::Transfer {
//         recipient: BUYER.into(),
//         amount: Uint128::new(1000),
//     };
//     execute(deps.as_mut(), mock_env(), info, send).unwrap();

//     // ensure balances updated
//     assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::zero());
//     assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

//     // second stake needs more to get next 1000 EPOXY
//     let info = mock_info(INVESTOR, &coins(1_500_000_000, DENOM));
//     execute(deps.as_mut(), mock_env(), info, buy).unwrap();

//     // ensure balances updated
//     assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
//     assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

//     // check curve info updated
//     let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
//     assert_eq!(curve.reserve, Uint128::new(2_000_000_000));
//     assert_eq!(curve.supply, Uint128::new(2000));
//     assert_eq!(curve.spot_price, Decimal::percent(200));

//     // check token info updated
//     let token = query_token_info(deps.as_ref()).unwrap();
//     assert_eq!(token.decimals, 2);
//     assert_eq!(token.total_supply, Uint128::new(2000));
// }

// #[test]
// fn bonding_fails_with_wrong_denom() {
//     let mut deps = mock_dependencies();
//     let curve_type = CurveType::Linear {
//         slope: Uint128::new(1),
//         scale: 1,
//     };
//     setup_test(deps.as_mut(), 2, 8, curve_type);

//     // fails when no tokens sent
//     let info = mock_info(INVESTOR, &[]);
//     let buy = ExecuteMsg::Buy {};
//     let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
//     assert_eq!(err, PaymentError::NoFunds {}.into());

//     // fails when wrong tokens sent
//     let info = mock_info(INVESTOR, &coins(1234567, "wei"));
//     let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
//     assert_eq!(err, PaymentError::MissingDenom(DENOM.into()).into());

//     // fails when too many tokens sent
//     let info = mock_info(INVESTOR, &[coin(3400022, DENOM), coin(1234567, "wei")]);
//     let err = execute(deps.as_mut(), mock_env(), info, buy).unwrap_err();
//     assert_eq!(err, PaymentError::MultipleDenoms {}.into());
// }

// #[test]
// fn burning_sends_reserve() {
//     let mut deps = mock_dependencies();
//     let curve_type = CurveType::Linear {
//         slope: Uint128::new(1),
//         scale: 1,
//     };
//     setup_test(deps.as_mut(), 2, 8, curve_type.clone());

//     // succeeds with proper token (20 BTC = 20*10^8 satoshi)
//     let info = mock_info(INVESTOR, &coins(2_000_000_000, DENOM));
//     let buy = ExecuteMsg::Buy {};
//     execute(deps.as_mut(), mock_env(), info, buy).unwrap();

//     // bob got 2000 EPOXY (20.00)
//     assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(2000));

//     // cannot burn too much
//     let info = mock_info(INVESTOR, &[]);
//     let burn = ExecuteMsg::Burn {
//         amount: Uint128::new(3000),
//     };
//     let err = execute(deps.as_mut(), mock_env(), info, burn).unwrap_err();
//     // TODO check error

//     // burn 1000 EPOXY to get back 15BTC (*10^8)
//     let info = mock_info(INVESTOR, &[]);
//     let burn = ExecuteMsg::Burn {
//         amount: Uint128::new(1000),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, burn).unwrap();

//     // balance is lower
//     assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));

//     // ensure we got our money back
//     assert_eq!(1, res.messages.len());
//     assert_eq!(
//         &res.messages[0],
//         &SubMsg::new(BankMsg::Send {
//             to_address: INVESTOR.into(),
//             amount: coins(1_500_000_000, DENOM),
//         })
//     );

//     // check curve info updated
//     let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
//     assert_eq!(curve.reserve, Uint128::new(500_000_000));
//     assert_eq!(curve.supply, Uint128::new(1000));
//     assert_eq!(curve.spot_price, Decimal::percent(100));

//     // check token info updated
//     let token = query_token_info(deps.as_ref()).unwrap();
//     assert_eq!(token.decimals, 2);
//     assert_eq!(token.total_supply, Uint128::new(1000));
// }
