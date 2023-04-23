use crate::abc::{CommonsPhase, CurveFn, MinMax};
use crate::contract::CwAbcResult;
use crate::ContractError;
use cosmwasm_std::{
    coins, ensure, Addr, BankMsg, Decimal as StdDecimal, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, StdError, StdResult, Storage, Uint128,
};
use cw_utils::must_pay;
use std::collections::HashSet;
use std::ops::Deref;
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery, TokenMsg};

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
            calculate_reserved_and_funded(payment, phase_config.open.allocation_percentage)
        }
        CommonsPhase::Closed => {
            return Err(ContractError::CommonsClosed {});
        }
    };

    // calculate how many tokens can be purchased with this and mint them
    let curve = curve_fn(curve_state.clone().decimals);
    curve_state.reserve += reserved;
    curve_state.funding += funded;
    // Calculate the supply based on the reserve
    let new_supply = curve.supply(curve_state.reserve);
    let minted = new_supply
        .checked_sub(curve_state.supply)
        .map_err(StdError::overflow)?;
    curve_state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    let mint_msg = mint_supply_msg(deps.storage, minted, &info.sender)?;

    Ok(Response::new()
        .add_message(mint_msg)
        .add_attribute("action", "buy")
        .add_attribute("from", info.sender)
        .add_attribute("reserved", reserved)
        .add_attribute("funded", funded)
        .add_attribute("supply", minted))
}

/// Build a message to mint the supply token to the sender
fn mint_supply_msg(storage: &dyn Storage, minted: Uint128, minter: &Addr) -> CwAbcResult<TokenMsg> {
    let denom = SUPPLY_DENOM.load(storage)?;
    // mint supply token
    Ok(TokenMsg::mint_contract_tokens(
        denom,
        minted,
        minter.to_string(),
    ))
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
) -> CwAbcResult {
    let burner = info.sender.clone();

    let supply_denom = SUPPLY_DENOM.load(deps.storage)?;
    let burn_amount = must_pay(&info, &supply_denom)?;
    // Burn the sent supply tokens
    let burn_msg = TokenMsg::burn_contract_tokens(supply_denom, burn_amount, burner.to_string());

    let taxed_amount = calculate_exit_tax(deps.storage, burn_amount)?;

    let mut curve_state = CURVE_STATE.load(deps.storage)?;
    let curve = curve_fn(curve_state.clone().decimals);

    // Reduce the supply by the amount burned
    curve_state.supply = curve_state
        .supply
        .checked_sub(burn_amount)
        .map_err(StdError::overflow)?;

    // Calculate the new reserve based on the new supply
    let new_reserve = curve.reserve(curve_state.supply);
    curve_state.reserve = new_reserve;
    curve_state.funding += taxed_amount;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    // Calculate how many reserve tokens to release based on the sell amount
    let released_reserve = curve_state
        .reserve
        .checked_sub(new_reserve)
        .map_err(StdError::overflow)?;

    // Now send the tokens to the sender
    let msg = BankMsg::Send {
        to_address: burner.to_string(),
        amount: coins(released_reserve.u128(), curve_state.reserve_denom),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_message(burn_msg)
        .add_attribute("action", "burn")
        .add_attribute("from", burner)
        .add_attribute("amount", burn_amount)
        .add_attribute("burned", released_reserve)
        .add_attribute("funded", taxed_amount))
}

/// Calculate the exit taxation for the sell amount based on the phase
fn calculate_exit_tax(storage: &dyn Storage, sell_amount: Uint128) -> CwAbcResult<Uint128> {
    // Load the phase config and phase
    let phase = PHASE.load(storage)?;
    let phase_config = PHASE_CONFIG.load(storage)?;

    // Calculate the exit tax based on the phase
    let exit_tax = match &phase {
        CommonsPhase::Hatch => phase_config.hatch.exit_tax,
        CommonsPhase::Open => phase_config.open.exit_tax,
        CommonsPhase::Closed => return Err(ContractError::CommonsClosed {}),
    };

    // TODO: safe decimal multiplication
    let taxed_amount = sell_amount * exit_tax;
    Ok(taxed_amount)
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

/// Add and remove addresses from the hatcher allowlist
pub fn update_hatch_allowlist(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    to_add: Vec<String>,
    to_remove: Vec<String>,
) -> CwAbcResult {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let mut allowlist = HATCHER_ALLOWLIST.may_load(deps.storage)?;

    if allowlist.is_none() {
        allowlist = Some(HashSet::new());
    }

    let allowlist = allowlist.as_mut().unwrap();

    // Add addresses to the allowlist
    for allow in to_add {
        let addr = deps.api.addr_validate(allow.as_str())?;
        allowlist.insert(addr);
    }

    // Remove addresses from the allowlist
    for deny in to_remove {
        let addr = deps.api.addr_validate(deny.as_str())?;
        allowlist.remove(&addr);
    }

    HATCHER_ALLOWLIST.save(deps.storage, allowlist)?;

    Ok(Response::new().add_attributes(vec![("action", "update_hatch_allowlist")]))
}

/// Update the hatch config
pub fn update_hatch_config(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    initial_raise: Option<MinMax>,
    initial_allocation_ratio: Option<StdDecimal>,
) -> CwAbcResult {
    // Assert that the sender is the contract owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Ensure we're in the Hatch phase
    PHASE.load(deps.storage)?.expect_hatch()?;

    // Load the current phase config
    let mut phase_config = PHASE_CONFIG.load(deps.storage)?;

    // Update the hatch config if new values are provided
    if let Some(initial_raise) = initial_raise {
        phase_config.hatch.initial_raise = initial_raise;
    }
    if let Some(initial_allocation_ratio) = initial_allocation_ratio {
        phase_config.hatch.initial_allocation_ratio = initial_allocation_ratio;
    }

    phase_config.hatch.validate()?;
    PHASE_CONFIG.save(deps.storage, &phase_config)?;

    Ok(Response::new().add_attribute("action", "update_hatch_config"))
}

/// Update the ownership of the contract
pub fn update_ownership(
    deps: DepsMut<TokenFactoryQuery>,
    env: &Env,
    info: &MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let ownership = cw_ownable::update_ownership(
        DepsMut {
            storage: deps.storage,
            api: deps.api,
            querier: QuerierWrapper::new(deps.querier.deref()),
        },
        &env.block,
        &info.sender,
        action,
    )?;

    Ok(Response::default().add_attributes(ownership.into_attributes()))
}