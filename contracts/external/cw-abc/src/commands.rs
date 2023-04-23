use std::collections::HashSet;
use cosmwasm_std::{Addr, BankMsg, coins, DepsMut, ensure, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128};
use token_bindings::{TokenFactoryQuery, TokenMsg};
use cw_utils::must_pay;
use crate::abc::{CommonsPhase, CurveFn};
use crate::ContractError;
use crate::contract::CwAbcResult;

use crate::state::{CURVE_STATE, DONATIONS, HATCHER_ALLOWLIST, HATCHERS, PHASE, PHASE_CONFIG, SUPPLY_DENOM};

pub fn execute_buy(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    curve_fn: CurveFn,
) -> CwAbcResult {
    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    let payment = must_pay(&info, &curve_state.reserve_denom)?;

    // Load the phase config and phase
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let mut phase = PHASE.load(deps.storage)?;

    let (reserved, funded) = match phase {
        CommonsPhase::Hatch => {
            let hatch_config = &phase_config.hatch;

            // Check that the potential hatcher is allowlisted
            assert_allowlisted(deps.storage, &info.sender)?;
            HATCHERS.update(deps.storage, |mut hatchers| -> StdResult<_>{
                hatchers.insert(info.sender.clone());
                Ok(hatchers)
            })?;

            // Check if the initial_raise max has been met
            if curve_state.reserve + payment >= hatch_config.initial_raise.max {
                // Transition to the Open phase, the hatchers' tokens are now vesting
                phase = CommonsPhase::Open;
                PHASE.save(deps.storage, &phase)?;
            }

            // Calculate the number of tokens sent to the funding pool using the initial allocation percentage
            // TODO: is it safe to multiply a Decimal with a Uint128?
            let funded = payment * hatch_config.initial_allocation_ratio;
            // Calculate the number of tokens sent to the reserve
            let reserved = payment - funded;

            (reserved, funded)
        }
        CommonsPhase::Open => {
            let open_config = &phase_config.open;

            // Calculate the number of tokens sent to the funding pool using the allocation percentage
            let funded = payment * open_config.allocation_percentage;
            // Calculate the number of tokens sent to the reserve
            let reserved = payment.checked_sub(funded).map_err(StdError::overflow)?;

            (reserved, funded)
        }
        CommonsPhase::Closed => {
            // TODO: what to do here?
            return Err(ContractError::CommonsClosed {});
        }
    };

    // calculate how many tokens can be purchased with this and mint them
    let curve = curve_fn(curve_state.clone().decimals);
    curve_state.reserve += reserved;
    curve_state.funding += funded;
    let new_supply = curve.supply(curve_state.reserve);
    let minted = new_supply
        .checked_sub(curve_state.supply)
        .map_err(StdError::overflow)?;
    curve_state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    let denom = SUPPLY_DENOM.load(deps.storage)?;
    // mint supply token
    let mint_msg = TokenMsg::MintTokens {
        denom,
        amount: minted,
        mint_to_address: info.sender.to_string(),
    };

    Ok(Response::new()
        .add_message(mint_msg)
        .add_attribute("action", "buy")
        .add_attribute("from", info.sender)
        .add_attribute("reserved", reserved)
        .add_attribute("funded", funded)
        .add_attribute("supply", minted))
}

pub fn execute_sell(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    curve_fn: CurveFn,
    amount: Uint128,
) -> CwAbcResult {
    let receiver = info.sender.clone();

    let denom = SUPPLY_DENOM.load(deps.storage)?;
    let payment = must_pay(&info, &denom)?;

    // calculate how many tokens can be purchased with this and mint them
    let mut state = CURVE_STATE.load(deps.storage)?;
    let curve = curve_fn(state.clone().decimals);
    state.supply = state
        .supply
        .checked_sub(amount)
        .map_err(StdError::overflow)?;
    let new_reserve = curve.reserve(state.supply);
    state.reserve = new_reserve;
    let released_funds = state
        .reserve
        .checked_sub(new_reserve)
        .map_err(StdError::overflow)?;

    // Load the phase config and phase
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let phase = PHASE.load(deps.storage)?;
    let (released, funded) = match phase {
        CommonsPhase::Hatch => {
            // TODO: perhaps we should allow selling during hatch phase with a larger exit tax?
            return Err(ContractError::HatchSellingDisabled {});
        }
        CommonsPhase::Open => {
            let open_config = &phase_config.open;

            // Calculate the number of tokens sent to the funding pool using the allocation percentage
            // TODO: unsafe multiplication
            let exit_tax = released_funds * open_config.exit_tax;
            // Calculate the number of tokens sent to the reserve
            let released = payment.checked_sub(exit_tax).map_err(StdError::overflow)?;

            (released, exit_tax)
        }
        CommonsPhase::Closed => {
            // TODO: what to do here?
            return Err(ContractError::CommonsClosed {});
        }
    };

    // Add the exit tax to the funding, reserve is already correctly calculated
    state.funding += funded;
    CURVE_STATE.save(deps.storage, &state)?;

    // Burn the tokens
    let burn_msg = TokenMsg::BurnTokens {
        denom,
        amount: payment,
        burn_from_address: info.sender.to_string(),
    };

    // now send the tokens to the sender (TODO: for sell_from we do something else, right???)
    let msg = BankMsg::Send {
        to_address: receiver.to_string(),
        amount: coins(released.u128(), state.reserve_denom),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_message(burn_msg)
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("released", released)
        .add_attribute("funded", funded)
    )
}

/// Send a donation to the funding pool
pub fn execute_donate(deps: DepsMut<TokenFactoryQuery>, _env: Env, info: MessageInfo) -> CwAbcResult {
    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    let payment = must_pay(&info, &curve_state.reserve_denom)?;
    curve_state.funding += payment;

    // No minting of tokens is necessary, the supply stays the same
    DONATIONS.save(deps.storage, &info.sender, &payment)?;

    Ok(Response::new()
        .add_attribute("action", "donate")
        .add_attribute("from", info.sender)
        .add_attribute("funded", payment))
}

/// Check if the sender is allowlisted for the hatch phase
fn assert_allowlisted(storage: &dyn Storage, hatcher: &Addr) -> Result<(), ContractError> {
    let allowlist = HATCHER_ALLOWLIST.may_load(storage)?;
    if let Some(allowlist) = allowlist {
        ensure!(
                allowlist.contains(hatcher),
                ContractError::SenderNotAllowlisted {
                    sender: hatcher.to_string(),
                }
            );
    }

    Ok(())
}

pub fn update_hatch_allowlist(deps: DepsMut<TokenFactoryQuery>, to_add: Vec<String>, to_remove: Vec<String>) -> CwAbcResult {
    let mut allowlist = HATCHER_ALLOWLIST.may_load(deps.storage)?;

    if let Some(ref mut allowlist) = allowlist {
        for allow in to_add {
            let addr = deps.api.addr_validate(allow.as_str())?;
            allowlist.insert(addr);
        }
        for deny in to_remove {
            let addr = deps.api.addr_validate(deny.as_str())?;
            allowlist.remove(&addr);
        }
    } else {
        let validated = to_add.into_iter()
            .map(|addr| deps.api.addr_validate(addr.as_str())).collect::<StdResult<HashSet<_>>>()?;
        allowlist = Some(validated);
    }

    HATCHER_ALLOWLIST.save(deps.storage, &allowlist.unwrap())?;

    Ok(Response::new().add_attributes(vec![("action", "update_hatch_allowlist")]))
}

