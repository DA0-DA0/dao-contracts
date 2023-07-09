#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use std::collections::HashSet;

use token_bindings::{TokenFactoryMsg, TokenFactoryQuery, TokenMsg};

use crate::abc::{CommonsPhase, CurveFn};
use crate::curves::DecimalPlaces;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdatePhaseConfigMsg};
use crate::state::{
    CurveState, CURVE_STATE, CURVE_TYPE, HATCHER_ALLOWLIST, PHASE, PHASE_CONFIG, SUPPLY_DENOM,
};
use crate::{commands, queries};
use cw_utils::nonpayable;

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-abc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// By default, the prefix for token factory tokens is "factory"
const DENOM_PREFIX: &str = "factory";

pub type CwAbcResult<T = Response<TokenFactoryMsg>> = Result<T, ContractError>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> CwAbcResult {
    nonpayable(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let InstantiateMsg {
        supply,
        reserve,
        curve_type,
        phase_config,
        hatcher_allowlist,
    } = msg;

    if supply.subdenom.is_empty() {
        return Err(ContractError::SupplyTokenError(
            "Token subdenom must not be empty.".to_string(),
        ));
    }

    phase_config.validate()?;

    // Create supply denom with metadata
    let create_supply_denom_msg = TokenMsg::CreateDenom {
        subdenom: supply.subdenom.clone(),
        metadata: Some(supply.metadata),
    };

    // TODO validate denom?

    // Save the denom
    SUPPLY_DENOM.save(
        deps.storage,
        &format!(
            "{}/{}/{}",
            DENOM_PREFIX,
            env.contract.address.into_string(),
            supply.subdenom
        ),
    )?;

    // Save the curve type and state
    let normalization_places = DecimalPlaces::new(supply.decimals, reserve.decimals);
    let curve_state = CurveState::new(reserve.denom, normalization_places);
    CURVE_STATE.save(deps.storage, &curve_state)?;
    CURVE_TYPE.save(deps.storage, &curve_type)?;

    if let Some(allowlist) = hatcher_allowlist {
        let allowlist = allowlist
            .into_iter()
            .map(|addr| deps.api.addr_validate(addr.as_str()))
            .collect::<StdResult<HashSet<_>>>()?;
        HATCHER_ALLOWLIST.save(deps.storage, &allowlist)?;
    }

    PHASE_CONFIG.save(deps.storage, &phase_config)?;

    // TODO don't hardcode this?
    PHASE.save(deps.storage, &CommonsPhase::Hatch)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    Ok(Response::default().add_message(create_supply_denom_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> CwAbcResult {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_execute
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_execute(deps, env, info, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in InstantiateMsg and stored in state, but you may want
/// to use custom math not included - make this easily reusable
pub fn do_execute(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
    curve_fn: CurveFn,
) -> CwAbcResult {
    match msg {
        ExecuteMsg::Buy {} => commands::execute_buy(deps, env, info, curve_fn),
        ExecuteMsg::Burn {} => commands::execute_sell(deps, env, info, curve_fn),
        ExecuteMsg::Donate {} => commands::execute_donate(deps, env, info),
        ExecuteMsg::UpdateHatchAllowlist { to_add, to_remove } => {
            commands::update_hatch_allowlist(deps, info, to_add, to_remove)
        }
        ExecuteMsg::UpdatePhaseConfig(update) => match update {
            UpdatePhaseConfigMsg::Hatch {
                initial_raise,
                initial_allocation_ratio,
            } => commands::update_hatch_config(
                deps,
                env,
                info,
                initial_raise,
                initial_allocation_ratio,
            ),
            _ => todo!(),
        },
        ExecuteMsg::UpdateOwnership(action) => {
            commands::update_ownership(deps, &env, &info, action)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<TokenFactoryQuery>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_execute
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_query(deps, env, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in [`InstantiateMsg`] and stored in state, but you may want
/// to use custom math not included - make this easily reusable
pub fn do_query(
    deps: Deps<TokenFactoryQuery>,
    _env: Env,
    msg: QueryMsg,
    curve_fn: CurveFn,
) -> StdResult<Binary> {
    match msg {
        // custom queries
        QueryMsg::CurveInfo {} => to_binary(&queries::query_curve_info(deps, curve_fn)?),
        QueryMsg::PhaseConfig {} => to_binary(&queries::query_phase_config(deps)?),
        QueryMsg::Donations { start_after, limit } => {
            to_binary(&queries::query_donations(deps, start_after, limit)?)
        }
        QueryMsg::Hatchers { start_after, limit } => {
            to_binary(&queries::query_hatchers(deps, start_after, limit)?)
        }
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
        // QueryMsg::GetDenom {
        //     creator_address,
        //     subdenom,
        // } => to_binary(&get_denom(deps, creator_address, subdenom)),
    }
}

// fn validate_denom(
//     deps: DepsMut<TokenFactoryQuery>,
//     denom: String,
// ) -> Result<(), TokenFactoryError> {
//     let denom_to_split = denom.clone();
//     let tokenfactory_denom_parts: Vec<&str> = denom_to_split.split('/').collect();

//     if tokenfactory_denom_parts.len() != 3 {
//         return Result::Err(TokenFactoryError::InvalidDenom {
//             denom,
//             message: std::format!(
//                 "denom must have 3 parts separated by /, had {}",
//                 tokenfactory_denom_parts.len()
//             ),
//         });
//     }

//     let prefix = tokenfactory_denom_parts[0];
//     let creator_address = tokenfactory_denom_parts[1];
//     let subdenom = tokenfactory_denom_parts[2];

//     if !prefix.eq_ignore_ascii_case("factory") {
//         return Result::Err(TokenFactoryError::InvalidDenom {
//             denom,
//             message: std::format!("prefix must be 'factory', was {}", prefix),
//         });
//     }

//     // Validate denom by attempting to query for full denom
//     let response = TokenQuerier::new(&deps.querier)
//         .full_denom(String::from(creator_address), String::from(subdenom));
//     if response.is_err() {
//         return Result::Err(TokenFactoryError::InvalidDenom {
//             denom,
//             message: response.err().unwrap().to_string(),
//         });
//     }

//     Result::Ok(())
// }

// this is poor man's "skip" flag
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::abc::CurveType;
    use crate::queries::query_curve_info;
    use cosmwasm_std::{
        testing::{mock_env, mock_info},
        CosmosMsg, Decimal, Uint128,
    };
    use speculoos::prelude::*;

    use crate::testing::*;

    //     fn get_balance<U: Into<String>>(deps: Deps, addr: U) -> Uint128 {
    //         query_balance(deps, addr.into()).unwrap().balance
    //     }

    //     fn setup_test(deps: DepsMut, decimals: u8, reserve_decimals: u8, curve_type: CurveType) {
    //         // this matches `linear_curve` test case from curves.rs
    //         let creator = String::from(CREATOR);
    //         let msg = default_instantiate(decimals, reserve_decimals, curve_type);
    //         let info = mock_info(&creator, &[]);

    //         // make sure we can instantiate with this
    //         let res = instantiate(deps, mock_env(), info, msg).unwrap();
    //         assert_eq!(0, res.messages.len());
    //     }

    /// Mock token factory querier dependencies

    #[test]
    fn proper_instantiation() -> CwAbcResult<()> {
        let mut deps = mock_tf_dependencies();

        // this matches `linear_curve` test case from curves.rs
        let creator = String::from("creator");
        let curve_type = CurveType::SquareRoot {
            slope: Uint128::new(1),
            scale: 1,
        };
        let msg = default_instantiate_msg(2, 8, curve_type.clone());
        let info = mock_info(&creator, &[]);

        // make sure we can instantiate with this
        let res = instantiate(deps.as_mut(), mock_env(), info, msg)?;
        assert_that!(res.messages.len()).is_equal_to(1);
        let submsg = res.messages.get(0).unwrap();
        assert_that!(submsg.msg).is_equal_to(CosmosMsg::Custom(TokenFactoryMsg::Token(
            TokenMsg::CreateDenom {
                subdenom: TEST_SUPPLY_DENOM.to_string(),
                metadata: Some(default_supply_metadata()),
            },
        )));

        // TODO!
        // // token info is proper
        // let token = query_token_info(deps.as_ref()).unwrap();
        // assert_that!(&token.name, &msg.name);
        // assert_that!(&token.symbol, &msg.symbol);
        // assert_that!(token.decimals, 2);
        // assert_that!(token.total_supply, Uint128::zero());

        // curve state is sensible
        let state = query_curve_info(deps.as_ref(), curve_type.to_curve_fn())?;
        assert_that!(state.reserve).is_equal_to(Uint128::zero());
        assert_that!(state.supply).is_equal_to(Uint128::zero());
        assert_that!(state.reserve_denom.as_str()).is_equal_to(TEST_RESERVE_DENOM);
        // spot price 0 as supply is 0
        assert_that!(state.spot_price).is_equal_to(Decimal::zero());

        // curve type is stored properly
        let curve = CURVE_TYPE.load(&deps.storage).unwrap();
        assert_eq!(curve_type, curve);

        // no balance
        // assert_eq!(get_balance(deps.as_ref(), &creator), Uint128::zero());

        Ok(())
    }

    //     #[test]
    //     fn buy_issues_tokens() {
    //         let mut deps = mock_dependencies();
    //         let curve_type = CurveType::Linear {
    //             slope: Uint128::new(1),
    //             scale: 1,
    //         };
    //         setup_test(deps.as_mut(), 2, 8, curve_type.clone());

    //         // succeeds with proper token (5 BTC = 5*10^8 satoshi)
    //         let info = mock_info(INVESTOR, &coins(500_000_000, DENOM));
    //         let buy = ExecuteMsg::Buy {};
    //         execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

    //         // bob got 1000 EPOXY (10.00)
    //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
    //         assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::zero());

    //         // send them all to buyer
    //         let info = mock_info(INVESTOR, &[]);
    //         let send = ExecuteMsg::Transfer {
    //             recipient: BUYER.into(),
    //             amount: Uint128::new(1000),
    //         };
    //         execute(deps.as_mut(), mock_env(), info, send).unwrap();

    //         // ensure balances updated
    //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::zero());
    //         assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

    //         // second stake needs more to get next 1000 EPOXY
    //         let info = mock_info(INVESTOR, &coins(1_500_000_000, DENOM));
    //         execute(deps.as_mut(), mock_env(), info, buy).unwrap();

    //         // ensure balances updated
    //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
    //         assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

    //         // check curve info updated
    //         let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
    //         assert_eq!(curve.reserve, Uint128::new(2_000_000_000));
    //         assert_eq!(curve.supply, Uint128::new(2000));
    //         assert_eq!(curve.spot_price, Decimal::percent(200));

    //         // check token info updated
    //         let token = query_token_info(deps.as_ref()).unwrap();
    //         assert_eq!(token.decimals, 2);
    //         assert_eq!(token.total_supply, Uint128::new(2000));
    //     }

    //     #[test]
    //     fn bonding_fails_with_wrong_denom() {
    //         let mut deps = mock_dependencies();
    //         let curve_type = CurveType::Linear {
    //             slope: Uint128::new(1),
    //             scale: 1,
    //         };
    //         setup_test(deps.as_mut(), 2, 8, curve_type);

    //         // fails when no tokens sent
    //         let info = mock_info(INVESTOR, &[]);
    //         let buy = ExecuteMsg::Buy {};
    //         let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
    //         assert_eq!(err, PaymentError::NoFunds {}.into());

    //         // fails when wrong tokens sent
    //         let info = mock_info(INVESTOR, &coins(1234567, "wei"));
    //         let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
    //         assert_eq!(err, PaymentError::MissingDenom(DENOM.into()).into());

    //         // fails when too many tokens sent
    //         let info = mock_info(INVESTOR, &[coin(3400022, DENOM), coin(1234567, "wei")]);
    //         let err = execute(deps.as_mut(), mock_env(), info, buy).unwrap_err();
    //         assert_eq!(err, PaymentError::MultipleDenoms {}.into());
    //     }

    //     #[test]
    //     fn burning_sends_reserve() {
    //         let mut deps = mock_dependencies();
    //         let curve_type = CurveType::Linear {
    //             slope: Uint128::new(1),
    //             scale: 1,
    //         };
    //         setup_test(deps.as_mut(), 2, 8, curve_type.clone());

    //         // succeeds with proper token (20 BTC = 20*10^8 satoshi)
    //         let info = mock_info(INVESTOR, &coins(2_000_000_000, DENOM));
    //         let buy = ExecuteMsg::Buy {};
    //         execute(deps.as_mut(), mock_env(), info, buy).unwrap();

    //         // bob got 2000 EPOXY (20.00)
    //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(2000));

    //         // cannot burn too much
    //         let info = mock_info(INVESTOR, &[]);
    //         let burn = ExecuteMsg::Burn {
    //             amount: Uint128::new(3000),
    //         };
    //         let err = execute(deps.as_mut(), mock_env(), info, burn).unwrap_err();
    //         // TODO check error

    //         // burn 1000 EPOXY to get back 15BTC (*10^8)
    //         let info = mock_info(INVESTOR, &[]);
    //         let burn = ExecuteMsg::Burn {
    //             amount: Uint128::new(1000),
    //         };
    //         let res = execute(deps.as_mut(), mock_env(), info, burn).unwrap();

    //         // balance is lower
    //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));

    //         // ensure we got our money back
    //         assert_eq!(1, res.messages.len());
    //         assert_eq!(
    //             &res.messages[0],
    //             &SubMsg::new(BankMsg::Send {
    //                 to_address: INVESTOR.into(),
    //                 amount: coins(1_500_000_000, DENOM),
    //             })
    //         );

    //         // check curve info updated
    //         let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
    //         assert_eq!(curve.reserve, Uint128::new(500_000_000));
    //         assert_eq!(curve.supply, Uint128::new(1000));
    //         assert_eq!(curve.spot_price, Decimal::percent(100));

    //         // check token info updated
    //         let token = query_token_info(deps.as_ref()).unwrap();
    //         assert_eq!(token.decimals, 2);
    //         assert_eq!(token.total_supply, Uint128::new(1000));
    //     }
}
