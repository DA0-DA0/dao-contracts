#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_tokenfactory_issuer::msg::{
    ExecuteMsg as IssuerExecuteMsg, InstantiateMsg as IssuerInstantiateMsg,
};
use cw_utils::{nonpayable, parse_reply_instantiate_data};
use std::collections::HashSet;
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery};

use crate::abc::{CommonsPhase, CurveFn};
use crate::curves::DecimalPlaces;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdatePhaseConfigMsg};
use crate::state::{
    CurveState, CURVE_STATE, CURVE_TYPE, HATCHER_ALLOWLIST, PHASE, PHASE_CONFIG, SUPPLY_DENOM,
    TOKEN_INSTANTIATION_INFO, TOKEN_ISSUER_CONTRACT,
};
use crate::{commands, queries};

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-abc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_FACTORY_ISSUER_REPLY_ID: u64 = 0;

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
        token_issuer_code_id,
    } = msg;

    if supply.subdenom.is_empty() {
        return Err(ContractError::SupplyTokenError(
            "Token subdenom must not be empty.".to_string(),
        ));
    }

    phase_config.validate()?;

    // Tnstantiate cw-token-factory-issuer contract
    // DAO (sender) is set as contract admin
    let issuer_instantiate_msg = SubMsg::reply_always(
        WasmMsg::Instantiate {
            admin: Some(info.sender.to_string()),
            code_id: token_issuer_code_id,
            msg: to_binary(&IssuerInstantiateMsg::NewToken {
                subdenom: supply.subdenom.clone(),
            })?,
            funds: info.funds,
            label: "cw-tokenfactory-issuer".to_string(),
        },
        INSTANTIATE_TOKEN_FACTORY_ISSUER_REPLY_ID,
    );

    // Save new token info for use in reply
    TOKEN_INSTANTIATION_INFO.save(deps.storage, &supply)?;

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

    Ok(Response::default().add_submessage(issuer_instantiate_msg))
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
        // TODO get token contract
        // QueryMsg::GetDenom {
        //     creator_address,
        //     subdenom,
        // } => to_binary(&get_denom(deps, creator_address, subdenom)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::<TokenFactoryMsg>::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    msg: Reply,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg.id {
        INSTANTIATE_TOKEN_FACTORY_ISSUER_REPLY_ID => {
            // Parse and save address of cw-tokenfactory-issuer
            let issuer_addr = parse_reply_instantiate_data(msg)?.contract_address;
            TOKEN_ISSUER_CONTRACT.save(deps.storage, &deps.api.addr_validate(&issuer_addr)?)?;

            // Load info for new token and remove temporary data
            let token_info = TOKEN_INSTANTIATION_INFO.load(deps.storage)?;
            TOKEN_INSTANTIATION_INFO.remove(deps.storage);

            // // Load the DAO address
            // let dao = DAO.load(deps.storage)?;

            // Format the denom and save it
            let denom = format!("factory/{}/{}", &issuer_addr, token_info.subdenom);

            SUPPLY_DENOM.save(deps.storage, &denom)?;

            // // Check supply is greater than zero, iterate through initial
            // // balances and sum them, add DAO balance as well.
            // let initial_supply = token
            //     .initial_balances
            //     .iter()
            //     .fold(Uint128::zero(), |previous, new_balance| {
            //         previous + new_balance.amount
            //     });
            // let total_supply = initial_supply + token.initial_dao_balance.unwrap_or_default();

            // // Cannot instantiate with no initial token owners because it would
            // // immediately lock the DAO.
            // if initial_supply.is_zero() {
            //     return Err(ContractError::InitialBalancesError {});
            // }

            // Msgs to be executed to finalize setup
            let mut msgs: Vec<WasmMsg> = vec![];

            // Grant an allowance to mint
            msgs.push(WasmMsg::Execute {
                contract_addr: issuer_addr.clone(),
                msg: to_binary(&IssuerExecuteMsg::SetMinterAllowance {
                    address: env.contract.address.to_string(),
                    // TODO let this be capped
                    allowance: Uint128::MAX,
                })?,
                funds: vec![],
            });

            // TODO fix metadata
            // // If metadata, set it by calling the contract
            // if let Some(metadata) = token_info.metadata {
            //     // The first denom_unit must be the same as the tf and base denom.
            //     // It must have an exponent of 0. This the smallest unit of the token.
            //     // For more info: // https://docs.cosmos.network/main/architecture/adr-024-coin-metadata
            //     let mut denom_units = vec![DenomUnit {
            //         denom: denom.clone(),
            //         exponent: 0,
            //         aliases: vec![token_info.subdenom],
            //     }];

            //     // Caller can optionally define additional units
            //     if let Some(mut additional_units) = metadata.additional_denom_units {
            //         denom_units.append(&mut additional_units);
            //     }

            //     // Sort denom units by exponent, must be in ascending order
            //     denom_units.sort_by(|a, b| a.exponent.cmp(&b.exponent));

            //     msgs.push(WasmMsg::Execute {
            //         contract_addr: issuer_addr.clone(),
            //         msg: to_binary(&IssuerExecuteMsg::SetDenomMetadata {
            //             metadata: Metadata {
            //                 description: metadata.description,
            //                 denom_units,
            //                 base: denom.clone(),
            //                 display: metadata.display,
            //                 name: metadata.name,
            //                 symbol: metadata.symbol,
            //             },
            //         })?,
            //         funds: vec![],
            //     });
            // }

            // TODO who should own the token contract?
            // // Update issuer contract owner to be the DAO
            // msgs.push(WasmMsg::Execute {
            //     contract_addr: issuer_addr.clone(),
            //     msg: to_binary(&IssuerExecuteMsg::ChangeContractOwner {
            //         new_owner: dao.to_string(),
            //     })?,
            //     funds: vec![],
            // });

            Ok(Response::new()
                .add_attribute("cw-tokenfactory-issuer-address", issuer_addr)
                .add_attribute("denom", denom)
                .add_messages(msgs))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

// #[cfg(test)]
// pub(crate) mod tests {
//     use super::*;
//     use crate::abc::CurveType;
//     use crate::queries::query_curve_info;
//     use cosmwasm_std::{
//         testing::{mock_env, mock_info},
//         CosmosMsg, Decimal, Uint128,
//     };
//     use speculoos::prelude::*;

//     use crate::testing::*;

//     //     fn get_balance<U: Into<String>>(deps: Deps, addr: U) -> Uint128 {
//     //         query_balance(deps, addr.into()).unwrap().balance
//     //     }

//     //     fn setup_test(deps: DepsMut, decimals: u8, reserve_decimals: u8, curve_type: CurveType) {
//     //         // this matches `linear_curve` test case from curves.rs
//     //         let creator = String::from(CREATOR);
//     //         let msg = default_instantiate(decimals, reserve_decimals, curve_type);
//     //         let info = mock_info(&creator, &[]);

//     //         // make sure we can instantiate with this
//     //         let res = instantiate(deps, mock_env(), info, msg).unwrap();
//     //         assert_eq!(0, res.messages.len());
//     //     }

//     /// Mock token factory querier dependencies

//     // #[test]
//     // fn proper_instantiation() -> CwAbcResult<()> {
//     //     let mut deps = mock_tf_dependencies();

//     //     // this matches `linear_curve` test case from curves.rs
//     //     let creator = String::from("creator");
//     //     let curve_type = CurveType::SquareRoot {
//     //         slope: Uint128::new(1),
//     //         scale: 1,
//     //     };
//     //     let msg = default_instantiate_msg(2, 8, curve_type.clone());
//     //     let info = mock_info(&creator, &[]);

//     //     // make sure we can instantiate with this
//     //     let res = instantiate(deps.as_mut(), mock_env(), info, msg)?;
//     //     assert_that!(res.messages.len()).is_equal_to(1);
//     //     let submsg = res.messages.get(0).unwrap();
//     //     assert_that!(submsg.msg).is_equal_to(CosmosMsg::Custom(TokenFactoryMsg::Token(
//     //         TokenMsg::CreateDenom {
//     //             subdenom: TEST_SUPPLY_DENOM.to_string(),
//     //             metadata: Some(default_supply_metadata()),
//     //         },
//     //     )));

//     //     // TODO!
//     //     // // token info is proper
//     //     // let token = query_token_info(deps.as_ref()).unwrap();
//     //     // assert_that!(&token.name, &msg.name);
//     //     // assert_that!(&token.symbol, &msg.symbol);
//     //     // assert_that!(token.decimals, 2);
//     //     // assert_that!(token.total_supply, Uint128::zero());

//     //     // curve state is sensible
//     //     let state = query_curve_info(deps.as_ref(), curve_type.to_curve_fn())?;
//     //     assert_that!(state.reserve).is_equal_to(Uint128::zero());
//     //     assert_that!(state.supply).is_equal_to(Uint128::zero());
//     //     assert_that!(state.reserve_denom.as_str()).is_equal_to(TEST_RESERVE_DENOM);
//     //     // spot price 0 as supply is 0
//     //     assert_that!(state.spot_price).is_equal_to(Decimal::zero());

//     //     // curve type is stored properly
//     //     let curve = CURVE_TYPE.load(&deps.storage).unwrap();
//     //     assert_eq!(curve_type, curve);

//     //     // no balance
//     //     // assert_eq!(get_balance(deps.as_ref(), &creator), Uint128::zero());

//     //     Ok(())
//     // }

//     //     #[test]
//     //     fn buy_issues_tokens() {
//     //         let mut deps = mock_dependencies();
//     //         let curve_type = CurveType::Linear {
//     //             slope: Uint128::new(1),
//     //             scale: 1,
//     //         };
//     //         setup_test(deps.as_mut(), 2, 8, curve_type.clone());

//     //         // succeeds with proper token (5 BTC = 5*10^8 satoshi)
//     //         let info = mock_info(INVESTOR, &coins(500_000_000, DENOM));
//     //         let buy = ExecuteMsg::Buy {};
//     //         execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

//     //         // bob got 1000 EPOXY (10.00)
//     //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
//     //         assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::zero());

//     //         // send them all to buyer
//     //         let info = mock_info(INVESTOR, &[]);
//     //         let send = ExecuteMsg::Transfer {
//     //             recipient: BUYER.into(),
//     //             amount: Uint128::new(1000),
//     //         };
//     //         execute(deps.as_mut(), mock_env(), info, send).unwrap();

//     //         // ensure balances updated
//     //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::zero());
//     //         assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

//     //         // second stake needs more to get next 1000 EPOXY
//     //         let info = mock_info(INVESTOR, &coins(1_500_000_000, DENOM));
//     //         execute(deps.as_mut(), mock_env(), info, buy).unwrap();

//     //         // ensure balances updated
//     //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
//     //         assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

//     //         // check curve info updated
//     //         let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
//     //         assert_eq!(curve.reserve, Uint128::new(2_000_000_000));
//     //         assert_eq!(curve.supply, Uint128::new(2000));
//     //         assert_eq!(curve.spot_price, Decimal::percent(200));

//     //         // check token info updated
//     //         let token = query_token_info(deps.as_ref()).unwrap();
//     //         assert_eq!(token.decimals, 2);
//     //         assert_eq!(token.total_supply, Uint128::new(2000));
//     //     }

//     //     #[test]
//     //     fn bonding_fails_with_wrong_denom() {
//     //         let mut deps = mock_dependencies();
//     //         let curve_type = CurveType::Linear {
//     //             slope: Uint128::new(1),
//     //             scale: 1,
//     //         };
//     //         setup_test(deps.as_mut(), 2, 8, curve_type);

//     //         // fails when no tokens sent
//     //         let info = mock_info(INVESTOR, &[]);
//     //         let buy = ExecuteMsg::Buy {};
//     //         let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
//     //         assert_eq!(err, PaymentError::NoFunds {}.into());

//     //         // fails when wrong tokens sent
//     //         let info = mock_info(INVESTOR, &coins(1234567, "wei"));
//     //         let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
//     //         assert_eq!(err, PaymentError::MissingDenom(DENOM.into()).into());

//     //         // fails when too many tokens sent
//     //         let info = mock_info(INVESTOR, &[coin(3400022, DENOM), coin(1234567, "wei")]);
//     //         let err = execute(deps.as_mut(), mock_env(), info, buy).unwrap_err();
//     //         assert_eq!(err, PaymentError::MultipleDenoms {}.into());
//     //     }

//     //     #[test]
//     //     fn burning_sends_reserve() {
//     //         let mut deps = mock_dependencies();
//     //         let curve_type = CurveType::Linear {
//     //             slope: Uint128::new(1),
//     //             scale: 1,
//     //         };
//     //         setup_test(deps.as_mut(), 2, 8, curve_type.clone());

//     //         // succeeds with proper token (20 BTC = 20*10^8 satoshi)
//     //         let info = mock_info(INVESTOR, &coins(2_000_000_000, DENOM));
//     //         let buy = ExecuteMsg::Buy {};
//     //         execute(deps.as_mut(), mock_env(), info, buy).unwrap();

//     //         // bob got 2000 EPOXY (20.00)
//     //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(2000));

//     //         // cannot burn too much
//     //         let info = mock_info(INVESTOR, &[]);
//     //         let burn = ExecuteMsg::Burn {
//     //             amount: Uint128::new(3000),
//     //         };
//     //         let err = execute(deps.as_mut(), mock_env(), info, burn).unwrap_err();
//     //         // TODO check error

//     //         // burn 1000 EPOXY to get back 15BTC (*10^8)
//     //         let info = mock_info(INVESTOR, &[]);
//     //         let burn = ExecuteMsg::Burn {
//     //             amount: Uint128::new(1000),
//     //         };
//     //         let res = execute(deps.as_mut(), mock_env(), info, burn).unwrap();

//     //         // balance is lower
//     //         assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));

//     //         // ensure we got our money back
//     //         assert_eq!(1, res.messages.len());
//     //         assert_eq!(
//     //             &res.messages[0],
//     //             &SubMsg::new(BankMsg::Send {
//     //                 to_address: INVESTOR.into(),
//     //                 amount: coins(1_500_000_000, DENOM),
//     //             })
//     //         );

//     //         // check curve info updated
//     //         let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
//     //         assert_eq!(curve.reserve, Uint128::new(500_000_000));
//     //         assert_eq!(curve.supply, Uint128::new(1000));
//     //         assert_eq!(curve.spot_price, Decimal::percent(100));

//     //         // check token info updated
//     //         let token = query_token_info(deps.as_ref()).unwrap();
//     //         assert_eq!(token.decimals, 2);
//     //         assert_eq!(token.total_supply, Uint128::new(1000));
//     //     }
// }
