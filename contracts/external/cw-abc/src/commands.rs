use crate::abc::{CommonsPhase, CurveFn};
use crate::contract::CwAbcResult;
use crate::ContractError;
use cosmwasm_std::{
    coins, ensure, Addr, BankMsg, Decimal as StdDecimal, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128,
};
use cw_utils::must_pay;
use std::collections::HashSet;
use token_bindings::{TokenFactoryQuery, TokenMsg};

use crate::state::{
    CURVE_STATE, DONATIONS, HATCHERS, HATCHER_ALLOWLIST, PHASE, PHASE_CONFIG, SUPPLY_DENOM,
};

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

    let (reserved, funded) = match &phase {
        CommonsPhase::Hatch => {
            let hatch_config = phase_config.hatch;
            // Check that the potential hatcher is allowlisted
            assert_allowlisted(deps.storage, &info.sender)?;
            update_hatcher_contributions(deps.storage, &info.sender, payment)?;

            // Check if the initial_raise max has been met
            if curve_state.reserve + payment >= hatch_config.initial_raise.max {
                // Transition to the Open phase, the hatchers' tokens are now vesting
                phase = CommonsPhase::Open;
                PHASE.save(deps.storage, &phase)?;
            }

            calculate_reserved_and_funded(payment, hatch_config.initial_allocation_ratio)
        }
        CommonsPhase::Open => {
            let open_config = phase_config.open;
            calculate_reserved_and_funded(payment, open_config.allocation_percentage)
        }
        CommonsPhase::Closed => {
            return Err(ContractError::CommonsClosed {});
        }
    };

    // calculate how many tokens can be purchased with this and mint them
    let curve = curve_fn(curve_state.clone().decimals);
    curve_state.reserve += reserved;
    curve_state.funding += funded;
    // Supply = locked + float
    let new_supply = curve.supply(curve_state.reserve + curve_state.funding);
    let minted = new_supply
        .checked_sub(curve_state.supply)
        .map_err(StdError::overflow)?;
    curve_state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    let denom = SUPPLY_DENOM.load(deps.storage)?;
    // mint supply token
    let mint_msg = TokenMsg::mint_contract_tokens(denom, minted, info.sender.to_string());

    Ok(Response::new()
        .add_message(mint_msg)
        .add_attribute("action", "buy")
        .add_attribute("from", info.sender)
        .add_attribute("reserved", reserved)
        .add_attribute("funded", funded)
        .add_attribute("supply", minted))
}

/// Return the reserved and funded amounts based on the payment and the allocation ratio
fn calculate_reserved_and_funded(
    payment: Uint128,
    allocation_ratio: StdDecimal,
) -> (Uint128, Uint128) {
    let funded = payment * allocation_ratio;
    let reserved = payment.checked_sub(funded).unwrap(); // Since allocation_ratio is < 1, this subtraction is safe
    (reserved, funded)
}

/// Add the hatcher's contribution to the total contributions
fn update_hatcher_contributions(
    storage: &mut dyn Storage,
    hatcher: &Addr,
    contribution: Uint128,
) -> StdResult<()> {
    HATCHERS.update(storage, hatcher, |amount| -> StdResult<_> {
        match amount {
            Some(mut amount) => {
                amount += contribution;
                Ok(amount)
            }
            None => Ok(contribution),
        }
    })?;
    Ok(())
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

    // Load the phase config and phase
    let phase = PHASE.load(deps.storage)?;
    let phase_config = PHASE_CONFIG.load(deps.storage)?;

    // calculate how many tokens can be purchased with this and mint them
    let mut state = CURVE_STATE.load(deps.storage)?;
    let curve = curve_fn(state.clone().decimals);

    // Calculate the exit tax based on the phase
    let exit_tax = match &phase {
        CommonsPhase::Hatch => phase_config.hatch.exit_tax,
        CommonsPhase::Open => phase_config.open.exit_tax,
        CommonsPhase::Closed => return Err(ContractError::CommonsClosed {}),
    };

    // TODO: safe decimal multiplication
    let taxed_amount = amount * exit_tax;
    let net_supply_reduction = amount
        .checked_sub(taxed_amount)
        .map_err(StdError::overflow)?;

    // Reduce the supply by the net amount
    state.supply = state
        .supply
        .checked_sub(net_supply_reduction)
        .map_err(StdError::overflow)?;
    let new_reserve = curve.reserve(state.supply);
    state.reserve = new_reserve;
    let released_funds = state
        .reserve
        .checked_sub(new_reserve)
        .map_err(StdError::overflow)?;

    // Add the exit tax to the funding, reserve is already correctly calculated
    state.funding += taxed_amount;
    CURVE_STATE.save(deps.storage, &state)?;

    // Burn the tokens
    let burn_msg = TokenMsg::burn_contract_tokens(denom, payment, info.sender.to_string());

    // Now send the tokens to the sender
    let msg = BankMsg::Send {
        to_address: receiver.to_string(),
        amount: coins(released_funds.u128(), state.reserve_denom),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_message(burn_msg)
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("released", released_funds)
        .add_attribute("funded", taxed_amount))
}

/// Send a donation to the funding pool
pub fn execute_donate(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
) -> CwAbcResult {
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

pub fn update_hatch_allowlist(
    deps: DepsMut<TokenFactoryQuery>,
    to_add: Vec<String>,
    to_remove: Vec<String>,
) -> CwAbcResult {
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
        let validated = to_add
            .into_iter()
            .map(|addr| deps.api.addr_validate(addr.as_str()))
            .collect::<StdResult<HashSet<_>>>()?;
        allowlist = Some(validated);
    }

    HATCHER_ALLOWLIST.save(deps.storage, &allowlist.unwrap())?;

    Ok(Response::new().add_attributes(vec![("action", "update_hatch_allowlist")]))
}
