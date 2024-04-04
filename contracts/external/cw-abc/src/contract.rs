#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_tokenfactory_issuer::msg::{
    DenomUnit, ExecuteMsg as IssuerExecuteMsg, InstantiateMsg as IssuerInstantiateMsg, Metadata,
};
use cw_utils::parse_reply_instantiate_data;
use dao_interface::token::{InitialBalance, TokenInfo};

use crate::abc::{CommonsPhase, CurveFn};
use crate::curves::DecimalPlaces;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    CurveState, CURVE_STATE, CURVE_TYPE, FEES_RECIPIENT, HATCHER_ALLOWLIST, MAX_SUPPLY,
    NEW_TOKEN_INFO, PHASE, PHASE_CONFIG, SUPPLY_DENOM, TOKEN_ISSUER_CONTRACT,
};
use crate::{commands, queries};

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-abc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_FACTORY_ISSUER_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let InstantiateMsg {
        fees_recipient,
        supply,
        reserve,
        curve_type,
        phase_config,
        hatcher_allowlist,
    } = msg;

    phase_config.validate()?;

    // Validate and store the fees recipient
    FEES_RECIPIENT.save(deps.storage, &deps.api.addr_validate(&fees_recipient)?)?;

    if let TokenInfo::New(new_token_info) = &supply.token_info {
        if new_token_info.subdenom.is_empty() {
            return Err(ContractError::SupplyTokenError(
                "Token subdenom must not be empty.".to_string(),
            ));
        }

        // Save new token info for use in reply
        NEW_TOKEN_INFO.save(deps.storage, new_token_info)?;
    }

    if let Some(max_supply) = supply.max_supply {
        MAX_SUPPLY.save(deps.storage, &max_supply)?;
    }

    // Save the curve type
    CURVE_TYPE.save(deps.storage, &curve_type)?;

    if let Some(allowlist) = hatcher_allowlist {
        for hatcher in allowlist {
            let hatcher = deps.api.addr_validate(&hatcher)?;

            if !HATCHER_ALLOWLIST.has(deps.storage, &hatcher) {
                HATCHER_ALLOWLIST.save(deps.storage, &hatcher, &Empty {})?;
            }
        }
    }

    PHASE_CONFIG.save(deps.storage, &phase_config)?;

    // TODO don't hardcode this? Make it configurable? Hatch config can be optional
    PHASE.save(deps.storage, &CommonsPhase::Hatch)?;

    // Initialize owner to sender
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    // Setup the curve state
    let normalization_places = DecimalPlaces::new(supply.decimals, reserve.decimals);
    let mut curve_state = CurveState::new(reserve.denom, normalization_places);

    let msgs = match supply.token_info {
        // Instantiate cw-token-factory-issuer contract if new
        TokenInfo::New(new_token_info) => vec![SubMsg::reply_always(
            WasmMsg::Instantiate {
                // Contract is immutable, no admin
                admin: None,
                code_id: new_token_info.token_issuer_code_id,
                msg: to_json_binary(&IssuerInstantiateMsg::NewToken {
                    subdenom: new_token_info.subdenom,
                })?,
                funds: info.funds,
                label: "cw-tokenfactory-issuer".to_string(),
            },
            INSTANTIATE_TOKEN_FACTORY_ISSUER_REPLY_ID,
        )],
        TokenInfo::Existing { denom } => {
            if !denom.starts_with("factory/") {
                return Err(ContractError::SupplyTokenError(
                    "Token must be issued by the tokenfactory".to_string(),
                ));
            }

            // 'factory/' length is 8, so we trim that off
            let issuer_subdenom = &denom[8..];

            // Get a validated issuer from the expected [issuer]/[subdenom] string
            let issuer = match issuer_subdenom.find('/') {
                Some(end_index) => {
                    let issuer = deps.api.addr_validate(&issuer_subdenom[..end_index])?;

                    // Set the existing supply on the curve state
                    let existing_supply = deps.querier.query_supply(&denom)?;

                    if let Some(max_supply) = supply.max_supply {
                        if existing_supply.amount > max_supply {
                            return Err(ContractError::CannotExceedMaxSupply { max: max_supply });
                        }
                    }

                    curve_state.supply = existing_supply.amount;

                    Ok(issuer)
                }
                None => Err(ContractError::SupplyTokenError(
                    "Tokenfactory denom did not contain a subdenom".to_string(),
                )),
            }?;

            TOKEN_ISSUER_CONTRACT.save(deps.storage, &issuer)?;

            vec![]
        }
        TokenInfo::Factory(_) => unimplemented!(),
    };

    // Save the curve state
    CURVE_STATE.save(deps.storage, &curve_state)?;

    Ok(Response::default().add_submessages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Buy {} => commands::execute_buy(deps, env, info),
        ExecuteMsg::Sell {} => commands::execute_sell(deps, env, info),
        ExecuteMsg::Close {} => commands::execute_close(deps, info),
        ExecuteMsg::Donate {} => commands::execute_donate(deps, env, info),
        ExecuteMsg::UpdateMaxSupply { max_supply } => {
            commands::update_max_supply(deps, info, max_supply)
        }
        ExecuteMsg::UpdateCurve { curve_type } => commands::update_curve(deps, info, curve_type),
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_execute
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_query(deps, env, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in [`InstantiateMsg`] and stored in state, but you may want
/// to use custom math not included - make this easily reusable
pub fn do_query(deps: Deps, _env: Env, msg: QueryMsg, curve_fn: CurveFn) -> StdResult<Binary> {
    match msg {
        // custom queries
        QueryMsg::CurveInfo {} => to_json_binary(&queries::query_curve_info(deps, curve_fn)?),
        QueryMsg::CurveType {} => to_json_binary(&CURVE_TYPE.load(deps.storage)?),
        QueryMsg::Denom {} => to_json_binary(&queries::get_denom(deps)?),
        QueryMsg::Donations { start_after, limit } => {
            to_json_binary(&queries::query_donations(deps, start_after, limit)?)
        }
        QueryMsg::FeesRecipient {} => to_json_binary(&FEES_RECIPIENT.load(deps.storage)?),
        QueryMsg::Hatchers { start_after, limit } => {
            to_json_binary(&queries::query_hatchers(deps, start_after, limit)?)
        }
        QueryMsg::HatcherAllowlist { start_after, limit } => {
            to_json_binary(&queries::query_hatcher_allowlist(deps, start_after, limit)?)
        }
        QueryMsg::MaxSupply {} => to_json_binary(&queries::query_max_supply(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::PhaseConfig {} => to_json_binary(&queries::query_phase_config(deps)?),
        QueryMsg::Phase {} => to_json_binary(&PHASE.load(deps.storage)?),
        QueryMsg::TokenContract {} => to_json_binary(&TOKEN_ISSUER_CONTRACT.load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_TOKEN_FACTORY_ISSUER_REPLY_ID => {
            // Parse and save address of cw-tokenfactory-issuer
            let issuer_addr = parse_reply_instantiate_data(msg)?.contract_address;
            TOKEN_ISSUER_CONTRACT.save(deps.storage, &deps.api.addr_validate(&issuer_addr)?)?;

            // Load info for new token and remove temporary data
            let new_token_info = NEW_TOKEN_INFO.load(deps.storage)?;
            NEW_TOKEN_INFO.remove(deps.storage);

            // Format the denom and save it
            // By default, the prefix for token factory tokens is "factory"
            let denom = format!("factory/{}/{}", &issuer_addr, new_token_info.subdenom);

            SUPPLY_DENOM.save(deps.storage, &denom)?;

            // Msgs to be executed to finalize setup
            let mut msgs: Vec<WasmMsg> = vec![
                // Grant an allowance to mint
                WasmMsg::Execute {
                    contract_addr: issuer_addr.clone(),
                    msg: to_json_binary(&IssuerExecuteMsg::SetMinterAllowance {
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
                    msg: to_json_binary(&IssuerExecuteMsg::SetBurnerAllowance {
                        address: env.contract.address.to_string(),
                        allowance: Uint128::MAX,
                    })?,
                    funds: vec![],
                },
            ];

            // If metadata, set it by calling the contract
            if let Some(metadata) = new_token_info.metadata {
                // The first denom_unit must be the same as the tf and base denom.
                // It must have an exponent of 0. This the smallest unit of the token.
                // For more info: // https://docs.cosmos.network/main/architecture/adr-024-coin-metadata
                let mut denom_units = vec![DenomUnit {
                    denom: denom.clone(),
                    exponent: 0,
                    aliases: vec![new_token_info.subdenom],
                }];

                // Caller can optionally define additional units
                if let Some(mut additional_units) = metadata.additional_denom_units {
                    denom_units.append(&mut additional_units);
                }

                // Sort denom units by exponent, must be in ascending order
                denom_units.sort_by(|a, b| a.exponent.cmp(&b.exponent));

                msgs.push(WasmMsg::Execute {
                    contract_addr: issuer_addr.clone(),
                    msg: to_json_binary(&IssuerExecuteMsg::SetDenomMetadata {
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

            // Check supply is greater than zero, iterate through initial
            // balances and sum them, add DAO balance as well.
            let initial_supply = new_token_info.initial_balances.iter().fold(
                new_token_info.initial_dao_balance.unwrap_or_default(),
                |previous, new_balance| previous + new_balance.amount,
            );

            if !initial_supply.is_zero() {
                if let Some(max_supply) = MAX_SUPPLY.may_load(deps.storage)? {
                    if initial_supply > max_supply {
                        return Err(ContractError::CannotExceedMaxSupply { max: max_supply });
                    }
                }

                // Grant an allowance to mint the initial supply
                msgs.push(WasmMsg::Execute {
                    contract_addr: issuer_addr.clone(),
                    msg: to_json_binary(&IssuerExecuteMsg::SetMinterAllowance {
                        address: env.contract.address.to_string(),
                        allowance: initial_supply,
                    })?,
                    funds: vec![],
                });

                // Call issuer contract to mint tokens for initial balances
                new_token_info
                    .initial_balances
                    .iter()
                    .for_each(|b: &InitialBalance| {
                        msgs.push(WasmMsg::Execute {
                            contract_addr: issuer_addr.clone(),
                            msg: to_json_binary(&IssuerExecuteMsg::Mint {
                                to_address: b.address.clone(),
                                amount: b.amount,
                            })
                            .unwrap_or_default(),
                            funds: vec![],
                        });
                    });

                // Add initial DAO balance to initial_balances if nonzero.
                if let Some(initial_dao_balance) = new_token_info.initial_dao_balance {
                    if !initial_dao_balance.is_zero() {
                        // In this case, it would be considered the fees recipient
                        let fees_recipient = FEES_RECIPIENT.load(deps.storage)?;

                        msgs.push(WasmMsg::Execute {
                            contract_addr: issuer_addr.clone(),
                            msg: to_json_binary(&IssuerExecuteMsg::Mint {
                                to_address: fees_recipient.to_string(),
                                amount: initial_dao_balance,
                            })?,
                            funds: vec![],
                        });
                    }
                }

                CURVE_STATE.update(deps.storage, |mut x| -> StdResult<_> {
                    x.supply = initial_supply;

                    Ok(x)
                })?;
            }

            Ok(Response::new()
                .add_attribute("cw-tokenfactory-issuer-address", issuer_addr)
                .add_attribute("denom", denom)
                .add_messages(msgs))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
