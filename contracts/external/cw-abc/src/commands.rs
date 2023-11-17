use cosmwasm_std::{
    ensure, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal as StdDecimal, DepsMut, Env,
    MessageInfo, QuerierWrapper, Response, StdError, StdResult, Storage, SubMsg, Uint128, WasmMsg,
};
use cw_tokenfactory_issuer::msg::ExecuteMsg as IssuerExecuteMsg;
use cw_utils::must_pay;
use std::collections::HashSet;
use std::ops::Deref;
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery};

use crate::abc::{CommonsPhase, CurveType};
use crate::contract::CwAbcResult;
use crate::msg::UpdatePhaseConfigMsg;
use crate::state::{
    CURVE_STATE, CURVE_TYPE, DONATIONS, HATCHERS, HATCHER_ALLOWLIST, MAX_SUPPLY, PHASE,
    PHASE_CONFIG, SUPPLY_DENOM, TOKEN_ISSUER_CONTRACT,
};
use crate::ContractError;

pub fn execute_buy(deps: DepsMut<TokenFactoryQuery>, _env: Env, info: MessageInfo) -> CwAbcResult {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();

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

            // Update hatcher contribution
            let contribution = update_hatcher_contributions(deps.storage, &info.sender, payment)?;

            // Check contribtuion is above minimum
            if contribution < hatch_config.contribution_limits.min {
                return Err(ContractError::ContributionLimit {
                    min: hatch_config.contribution_limits.min,
                    max: hatch_config.contribution_limits.max,
                });
            }

            // Check contribution is below maximum
            if contribution > hatch_config.contribution_limits.max {
                return Err(ContractError::ContributionLimit {
                    min: hatch_config.contribution_limits.min,
                    max: hatch_config.contribution_limits.max,
                });
            }

            // Check if the initial_raise max has been met
            if curve_state.reserve + payment >= hatch_config.initial_raise.max {
                // Transition to the Open phase
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

    // Calculate how many tokens can be purchased with this and mint them
    let curve = curve_fn(curve_state.clone().decimals);
    curve_state.reserve += reserved;
    curve_state.funding += funded;

    // Calculate the supply based on the reserve
    let new_supply = curve.supply(curve_state.reserve);
    let minted = new_supply
        .checked_sub(curve_state.supply)
        .map_err(StdError::overflow)?;

    // Check that the minted amount has not exceeded the max supply (if configured)
    if let Some(max_supply) = MAX_SUPPLY.may_load(deps.storage)? {
        if new_supply > max_supply {
            return Err(ContractError::CannotExceedMaxSupply { max: max_supply });
        }
    }

    // Save the new curve state
    curve_state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    // Mint tokens for sender by calling mint on the cw-tokenfactory-issuer contract
    let issuer_addr = TOKEN_ISSUER_CONTRACT.load(deps.storage)?;
    let mint_msg = WasmMsg::Execute {
        contract_addr: issuer_addr.to_string(),
        msg: to_binary(&IssuerExecuteMsg::Mint {
            to_address: info.sender.to_string(),
            amount: minted,
        })?,
        funds: vec![],
    };

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
) -> StdResult<Uint128> {
    HATCHERS.update(storage, hatcher, |amount| -> StdResult<_> {
        match amount {
            Some(mut amount) => {
                amount += contribution;
                Ok(amount)
            }
            None => Ok(contribution),
        }
    })
}

/// Sell tokens on the bonding curve
pub fn execute_sell(deps: DepsMut<TokenFactoryQuery>, _env: Env, info: MessageInfo) -> CwAbcResult {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();

    let supply_denom = SUPPLY_DENOM.load(deps.storage)?;
    let burn_amount = must_pay(&info, &supply_denom)?;

    let issuer_addr = TOKEN_ISSUER_CONTRACT.load(deps.storage)?;

    // Burn the sent supply tokens
    let burn_msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![
        // Send tokens to the issuer contract to be burned
        CosmosMsg::<TokenFactoryMsg>::Bank(BankMsg::Send {
            to_address: issuer_addr.to_string().clone(),
            amount: vec![Coin {
                amount: burn_amount,
                denom: supply_denom,
            }],
        }),
        // Execute burn on the cw-tokenfactory-issuer contract
        CosmosMsg::<TokenFactoryMsg>::Wasm(WasmMsg::Execute {
            contract_addr: issuer_addr.to_string(),
            msg: to_binary(&IssuerExecuteMsg::Burn {
                from_address: info.sender.to_string(),
                amount: burn_amount,
            })?,
            funds: vec![],
        }),
    ];

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
    let msg_send = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            amount: released_reserve,
            denom: curve_state.reserve_denom,
        }],
    }));

    Ok(Response::<TokenFactoryMsg>::new()
        .add_messages(burn_msgs)
        .add_submessage(msg_send)
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
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

    debug_assert!(
        exit_tax <= StdDecimal::percent(100),
        "Exit tax must be <= 100%"
    );

    // This won't ever overflow because it's checked
    let taxed_amount = sell_amount * exit_tax;
    Ok(taxed_amount)
}

/// Transitions the bonding curve to a closed phase where only sells are allowed
pub fn execute_close(deps: DepsMut<TokenFactoryQuery>, info: MessageInfo) -> CwAbcResult {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    PHASE.save(deps.storage, &CommonsPhase::Closed)?;

    Ok(Response::new().add_attribute("action", "close"))
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
    CURVE_STATE.save(deps.storage, &curve_state)?;

    // No minting of tokens is necessary, the supply stays the same
    DONATIONS.save(deps.storage, &info.sender, &payment)?;

    Ok(Response::new()
        .add_attribute("action", "donate")
        .add_attribute("donor", info.sender)
        .add_attribute("amount", payment))
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

/// Set the maxiumum supply (only callable by owner)
/// If `max_supply` is set to None there will be no limit.`
pub fn set_max_supply(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    max_supply: Option<Uint128>,
) -> CwAbcResult {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    match max_supply {
        Some(max) => MAX_SUPPLY.save(deps.storage, &max)?,
        None => MAX_SUPPLY.remove(deps.storage),
    }

    Ok(Response::new()
        .add_attribute("action", "set_max_supply")
        .add_attribute("value", max_supply.unwrap_or(Uint128::MAX).to_string()))
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

/// Update the configuration of a particular phase
pub fn update_phase_config(
    deps: DepsMut<TokenFactoryQuery>,
    _env: Env,
    info: MessageInfo,
    update_phase_config_msg: UpdatePhaseConfigMsg,
) -> CwAbcResult {
    // Assert that the sender is the contract owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Load phase and phase config
    let phase = PHASE.load(deps.storage)?;

    // Load the current phase config
    let mut phase_config = PHASE_CONFIG.load(deps.storage)?;

    match update_phase_config_msg {
        UpdatePhaseConfigMsg::Hatch {
            exit_tax,
            initial_raise,
            initial_allocation_ratio,
            contribution_limits,
        } => {
            // Check we are in the hatch phase
            phase.expect_hatch()?;

            // Update the hatch config if new values are provided
            if let Some(contribution_limits) = contribution_limits {
                phase_config.hatch.contribution_limits = contribution_limits;
            }
            if let Some(exit_tax) = exit_tax {
                phase_config.hatch.exit_tax = exit_tax;
            }
            if let Some(initial_raise) = initial_raise {
                phase_config.hatch.initial_raise = initial_raise;
            }
            if let Some(initial_allocation_ratio) = initial_allocation_ratio {
                phase_config.hatch.initial_allocation_ratio = initial_allocation_ratio;
            }

            // Validate config
            phase_config.hatch.validate()?;
            PHASE_CONFIG.save(deps.storage, &phase_config)?;

            Ok(Response::new().add_attribute("action", "update_hatch_phase_config"))
        }
        UpdatePhaseConfigMsg::Open {
            exit_tax,
            allocation_percentage,
        } => {
            // Check we are in the open phase
            phase.expect_open()?;

            // Update the hatch config if new values are provided
            if let Some(allocation_percentage) = allocation_percentage {
                phase_config.open.allocation_percentage = allocation_percentage;
            }
            if let Some(exit_tax) = exit_tax {
                phase_config.hatch.exit_tax = exit_tax;
            }

            // Validate config
            phase_config.open.validate()?;
            PHASE_CONFIG.save(deps.storage, &phase_config)?;

            Ok(Response::new().add_attribute("action", "update_open_phase_config"))
        }
        // TODO what should the closed phase configuration be, is there one?
        _ => todo!(),
    }
}

/// Update the bonding curve. Only callable by the owner.
/// NOTE: this changes the pricing. Use with caution.
/// TODO: what other limitations do we want to put on this?
pub fn update_curve(
    deps: DepsMut<TokenFactoryQuery>,
    info: MessageInfo,
    curve_type: CurveType,
) -> CwAbcResult {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    CURVE_TYPE.save(deps.storage, &curve_type)?;

    Ok(Response::new().add_attribute("action", "close"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::prelude::*;
    use cosmwasm_std::testing::*;

    mod donate {
        use super::*;
        use crate::abc::CurveType;
        use crate::testing::mock_init;
        use cosmwasm_std::coin;
        use cw_utils::PaymentError;

        const TEST_DONOR: &str = "donor";

        fn exec_donate(deps: DepsMut<TokenFactoryQuery>, donation_amount: u128) -> CwAbcResult {
            execute_donate(
                deps,
                mock_env(),
                mock_info(TEST_DONOR, &[coin(donation_amount, TEST_RESERVE_DENOM)]),
            )
        }

        #[test]
        fn should_fail_with_no_funds() -> CwAbcResult<()> {
            let mut deps = mock_tf_dependencies();
            let curve_type = CurveType::Linear {
                slope: Uint128::new(1),
                scale: 1,
            };
            let init_msg = default_instantiate_msg(2, 8, curve_type);
            mock_init(deps.as_mut(), init_msg)?;

            let res = exec_donate(deps.as_mut(), 0);
            assert_that!(res)
                .is_err()
                .is_equal_to(ContractError::Payment(PaymentError::NoFunds {}));

            Ok(())
        }

        #[test]
        fn should_fail_with_incorrect_denom() -> CwAbcResult<()> {
            let mut deps = mock_tf_dependencies();
            let curve_type = CurveType::Linear {
                slope: Uint128::new(1),
                scale: 1,
            };
            let init_msg = default_instantiate_msg(2, 8, curve_type);
            mock_init(deps.as_mut(), init_msg)?;

            let res = execute_donate(
                deps.as_mut(),
                mock_env(),
                mock_info(TEST_DONOR, &[coin(1, "fake")]),
            );
            assert_that!(res)
                .is_err()
                .is_equal_to(ContractError::Payment(PaymentError::MissingDenom(
                    TEST_RESERVE_DENOM.to_string(),
                )));

            Ok(())
        }

        #[test]
        fn should_add_to_funding_pool() -> CwAbcResult<()> {
            let mut deps = mock_tf_dependencies();
            // this matches `linear_curve` test case from curves.rs
            let curve_type = CurveType::SquareRoot {
                slope: Uint128::new(1),
                scale: 1,
            };
            let init_msg = default_instantiate_msg(2, 8, curve_type);
            mock_init(deps.as_mut(), init_msg)?;

            let donation_amount = 5;
            let _res = exec_donate(deps.as_mut(), donation_amount)?;

            // check that the curve's funding has been increased while supply and reserve have not
            let curve_state = CURVE_STATE.load(&deps.storage)?;
            assert_that!(curve_state.funding).is_equal_to(Uint128::new(donation_amount));

            // check that the donor is in the donations map
            let donation = DONATIONS.load(&deps.storage, &Addr::unchecked(TEST_DONOR))?;
            assert_that!(donation).is_equal_to(Uint128::new(donation_amount));

            Ok(())
        }
    }
}
