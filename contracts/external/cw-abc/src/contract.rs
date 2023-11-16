#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_tokenfactory_issuer::msg::{
    DenomUnit, ExecuteMsg as IssuerExecuteMsg, InstantiateMsg as IssuerInstantiateMsg, Metadata,
};
use cw_utils::parse_reply_instantiate_data;
use std::collections::HashSet;
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery};

use crate::abc::{CommonsPhase, CurveFn};
use crate::curves::DecimalPlaces;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdatePhaseConfigMsg};
use crate::state::{
    CurveState, CURVE_STATE, CURVE_TYPE, HATCHER_ALLOWLIST, MAX_SUPPLY, PHASE, PHASE_CONFIG,
    SUPPLY_DENOM, TOKEN_INSTANTIATION_INFO, TOKEN_ISSUER_CONTRACT,
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

    // Save new token info for use in reply
    TOKEN_INSTANTIATION_INFO.save(deps.storage, &supply)?;

    if let Some(max_supply) = supply.max_supply {
        MAX_SUPPLY.save(deps.storage, &max_supply)?;
    }

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

    // TODO don't hardcode this? Make it configurable? Hatch config can be optional
    PHASE.save(deps.storage, &CommonsPhase::Hatch)?;

    // Initialize owner to sender
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    // TODO Potential renounce admin?
    // Tnstantiate cw-token-factory-issuer contract
    // Sender is set as contract admin
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

    Ok(Response::default().add_submessage(issuer_instantiate_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> CwAbcResult {
    match msg {
        ExecuteMsg::Buy {} => commands::execute_buy(deps, env, info),
        ExecuteMsg::Burn {} => commands::execute_sell(deps, env, info),
        ExecuteMsg::Donate {} => commands::execute_donate(deps, env, info),
        ExecuteMsg::UpdateHatchAllowlist { to_add, to_remove } => {
            commands::update_hatch_allowlist(deps, info, to_add, to_remove)
        }
        ExecuteMsg::UpdatePhaseConfig(update_msg) => {
            commands::update_phase_config(deps, env, info, update_msg)
        }
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
        QueryMsg::Denom {} => to_binary(&queries::get_denom(deps)?),
        QueryMsg::TokenContract {} => to_binary(&TOKEN_ISSUER_CONTRACT.load(deps.storage)?),
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

            // Format the denom and save it
            let denom = format!("factory/{}/{}", &issuer_addr, token_info.subdenom);

            SUPPLY_DENOM.save(deps.storage, &denom)?;

            // Msgs to be executed to finalize setup
            let mut msgs: Vec<WasmMsg> = vec![
                // Grant an allowance to mint
                WasmMsg::Execute {
                    contract_addr: issuer_addr.clone(),
                    msg: to_binary(&IssuerExecuteMsg::SetMinterAllowance {
                        address: env.contract.address.to_string(),
                        // Allowance needs to be max as this the is the amount of tokens
                        // the minter is allowed to mint, not to be confused with max supply
                        // which we have to enforce elsewhere.
                        allowance: Uint128::MAX,
                    })?,
                    funds: vec![],
                },
                // Grant an allowance to burn
                WasmMsg::Execute {
                    contract_addr: issuer_addr.clone(),
                    msg: to_binary(&IssuerExecuteMsg::SetBurnerAllowance {
                        address: env.contract.address.to_string(),
                        allowance: Uint128::MAX,
                    })?,
                    funds: vec![],
                },
            ];

            // If metadata, set it by calling the contract
            if let Some(metadata) = token_info.metadata {
                // The first denom_unit must be the same as the tf and base denom.
                // It must have an exponent of 0. This the smallest unit of the token.
                // For more info: // https://docs.cosmos.network/main/architecture/adr-024-coin-metadata
                let mut denom_units = vec![DenomUnit {
                    denom: denom.clone(),
                    exponent: 0,
                    aliases: vec![token_info.subdenom],
                }];

                // Caller can optionally define additional units
                if let Some(mut additional_units) = metadata.additional_denom_units {
                    denom_units.append(&mut additional_units);
                }

                // Sort denom units by exponent, must be in ascending order
                denom_units.sort_by(|a, b| a.exponent.cmp(&b.exponent));

                msgs.push(WasmMsg::Execute {
                    contract_addr: issuer_addr.clone(),
                    msg: to_binary(&IssuerExecuteMsg::SetDenomMetadata {
                        metadata: Metadata {
                            description: metadata.description,
                            denom_units,
                            base: denom.clone(),
                            display: metadata.display,
                            name: metadata.name,
                            symbol: metadata.symbol,
                        },
                    })?,
                    funds: vec![],
                });
            }

            Ok(Response::new()
                .add_attribute("cw-tokenfactory-issuer-address", issuer_addr)
                .add_attribute("denom", denom)
                .add_messages(msgs))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
