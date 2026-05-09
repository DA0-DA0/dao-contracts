use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, StdResult, Storage, Uint128, WasmMsg,
};
use cw_tokenfactory_issuer::msg::ExecuteMsg as IssuerExecuteMsg;
use cw_utils::must_pay;
use std::ops::Deref;

use crate::abc::{CommonsPhase, CurveType, HatchConfig, MinMax};
use crate::helpers::{calculate_buy_quote, calculate_sell_quote};
use crate::msg::{HatcherAllowlistEntryMsg, UpdatePhaseConfigMsg};
use crate::state::{
    hatcher_allowlist, HatcherAllowlistConfig, HatcherAllowlistConfigType, CURVE_STATE, CURVE_TYPE,
    DONATIONS, FUNDING_POOL_FORWARDING, HATCHERS, HATCHER_DAO_PRIORITY_QUEUE, IS_PAUSED,
    MAX_SUPPLY, PHASE, PHASE_CONFIG, SUPPLY_DENOM, TOKEN_ISSUER_CONTRACT,
};
use crate::ContractError;

pub fn buy(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    let payment = must_pay(&info, &curve_state.reserve_denom)?;

    // Load the phase config and phase
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let mut phase = PHASE.load(deps.storage)?;

    // Calculate the curve state from the buy
    let buy_quote = calculate_buy_quote(payment, &curve_type, &curve_state, &phase, &phase_config)?;

    // Validate phase
    match &phase {
        CommonsPhase::Hatch => {
            // Check that the potential hatcher is allowlisted
            let hatch_config = assert_allowlisted(
                deps.querier,
                deps.storage,
                &info.sender,
                &phase_config.hatch,
            )?;

            // Update hatcher contribution
            let contribution =
                HATCHERS.update(deps.storage, &info.sender, |amount| -> StdResult<_> {
                    Ok(amount.unwrap_or_default() + payment)
                })?;

            // Check contribution is within limits
            if contribution < hatch_config.contribution_limits.min
                || contribution > hatch_config.contribution_limits.max
            {
                return Err(ContractError::ContributionLimit {
                    min: hatch_config.contribution_limits.min,
                    max: hatch_config.contribution_limits.max,
                });
            }

            // Check if the initial_raise max has been met
            if buy_quote.new_reserve >= hatch_config.initial_raise.max {
                // Transition to the Open phase
                phase = CommonsPhase::Open;

                // Can clean up state here
                hatcher_allowlist().clear(deps.storage);

                PHASE.save(deps.storage, &phase)?;
            }
        }
        CommonsPhase::Open => {}
        CommonsPhase::Closed => {
            return Err(ContractError::CommonsClosed {});
        }
    };

    // Check that the minted amount has not exceeded the max supply (if configured)
    if let Some(max_supply) = MAX_SUPPLY.may_load(deps.storage)? {
        if buy_quote.new_supply > max_supply {
            return Err(ContractError::CannotExceedMaxSupply { max: max_supply });
        }
    }

    // Mint tokens for sender by calling mint on the cw-tokenfactory-issuer contract
    let issuer_addr = TOKEN_ISSUER_CONTRACT.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: issuer_addr.to_string(),
        msg: to_json_binary(&IssuerExecuteMsg::Mint {
            to_address: info.sender.to_string(),
            amount: buy_quote.amount,
        })?,
        funds: vec![],
    })];

    // Send funding to fee recipient
    if buy_quote.funded > Uint128::zero() {
        if let Some(funding_pool_forwarding) = FUNDING_POOL_FORWARDING.may_load(deps.storage)? {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: funding_pool_forwarding.to_string(),
                amount: vec![Coin {
                    amount: buy_quote.funded,
                    denom: curve_state.reserve_denom.clone(),
                }],
            }))
        } else {
            curve_state.funding += buy_quote.funded;
        }
    };

    // Save the new curve state
    curve_state.supply = buy_quote.new_supply;
    curve_state.reserve = buy_quote.new_reserve;

    CURVE_STATE.save(deps.storage, &curve_state)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "buy")
        .add_attribute("from", info.sender)
        .add_attribute("amount", payment)
        .add_attribute("reserved", buy_quote.new_reserve)
        .add_attribute("minted", buy_quote.amount)
        .add_attribute("funded", buy_quote.funded)
        .add_attribute("supply", buy_quote.new_supply))
}

/// Sell tokens on the bonding curve
pub fn sell(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let supply_denom = SUPPLY_DENOM.load(deps.storage)?;
    let burn_amount = must_pay(&info, &supply_denom)?;

    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    // Load the phase configuration and the current phase
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let phase = PHASE.load(deps.storage)?;

    // Calculate the sell quote
    let sell_quote = calculate_sell_quote(
        burn_amount,
        &curve_type,
        &curve_state,
        &phase,
        &phase_config,
    )?;

    let mut send_msgs: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            amount: sell_quote.amount,
            denom: curve_state.reserve_denom.clone(),
        }],
    })];

    let issuer_addr = TOKEN_ISSUER_CONTRACT.load(deps.storage)?;

    // Burn the sent supply tokens
    let burn_msgs: Vec<CosmosMsg> = vec![
        // Send tokens to the issuer contract to be burned
        CosmosMsg::Bank(BankMsg::Send {
            to_address: issuer_addr.to_string().clone(),
            amount: vec![Coin {
                amount: burn_amount,
                denom: supply_denom,
            }],
        }),
        // Execute burn on the cw-tokenfactory-issuer contract
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: issuer_addr.to_string(),
            msg: to_json_binary(&IssuerExecuteMsg::Burn {
                from_address: issuer_addr.to_string(),
                amount: burn_amount,
            })?,
            funds: vec![],
        }),
    ];

    // Send exit fee to the funding pool
    if sell_quote.funded > Uint128::zero() {
        if let Some(funding_pool_forwarding) = FUNDING_POOL_FORWARDING.may_load(deps.storage)? {
            send_msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: funding_pool_forwarding.to_string(),
                amount: vec![Coin {
                    amount: sell_quote.funded,
                    denom: curve_state.reserve_denom.clone(),
                }],
            }))
        } else {
            curve_state.funding += sell_quote.funded;
        }
    }

    // Update the curve state
    curve_state.reserve = sell_quote.new_reserve;
    curve_state.supply = sell_quote.new_supply;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    Ok(Response::new()
        .add_messages(burn_msgs)
        .add_messages(send_msgs)
        .add_attribute("action", "sell")
        .add_attribute("from", info.sender)
        .add_attribute("amount", burn_amount)
        .add_attribute("reserved", sell_quote.new_reserve)
        .add_attribute("supply", sell_quote.new_supply)
        .add_attribute("burned", sell_quote.amount)
        .add_attribute("funded", sell_quote.funded))
}

/// Transitions the bonding curve to a closed phase where only sells are allowed
pub fn close(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    PHASE.save(deps.storage, &CommonsPhase::Closed)?;

    Ok(Response::new().add_attribute("action", "close"))
}

/// Send a donation to the funding pool
pub fn donate(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    let payment = must_pay(&info, &curve_state.reserve_denom)?;

    let msgs =
        if let Some(funding_pool_forwarding) = FUNDING_POOL_FORWARDING.may_load(deps.storage)? {
            vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: funding_pool_forwarding.to_string(),
                amount: info.funds,
            })]
        } else {
            curve_state.funding += payment;

            CURVE_STATE.save(deps.storage, &curve_state)?;

            vec![]
        };

    // No minting of tokens is necessary, the supply stays the same
    let total_donation =
        DONATIONS.update(deps.storage, &info.sender, |maybe_amount| -> StdResult<_> {
            if let Some(amount) = maybe_amount {
                Ok(amount.checked_add(payment)?)
            } else {
                Ok(payment)
            }
        })?;

    Ok(Response::new()
        .add_attribute("action", "donate")
        .add_attribute("donor", info.sender)
        .add_attribute("amount", payment)
        .add_attribute("total_donation", total_donation)
        .add_messages(msgs))
}

/// Withdraw funds from the funding pool (only callable by owner)
pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Validate ownership
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    // Get amount to withdraw
    let amount = amount.unwrap_or(curve_state.funding);

    // Construct the withdraw message
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: curve_state.reserve_denom.clone(),
            amount,
        }],
    });

    // Update the curve state
    curve_state.funding = curve_state.funding.checked_sub(amount)?;
    CURVE_STATE.save(deps.storage, &curve_state)?;

    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_attribute("withdrawer", info.sender)
        .add_attribute("amount", amount)
        .add_message(msg))
}

/// Updates the funding pool forwarding (only callable by owner)
pub fn update_funding_pool_forwarding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: Option<String>,
) -> Result<Response, ContractError> {
    // Validate ownership
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Update the funding pool forwarding
    match &address {
        Some(address) => {
            FUNDING_POOL_FORWARDING.save(deps.storage, &deps.api.addr_validate(address)?)?;
        }
        None => FUNDING_POOL_FORWARDING.remove(deps.storage),
    };

    Ok(Response::new()
        .add_attribute("action", "update_funding_pool_forwarding")
        .add_attribute(
            "address",
            address.unwrap_or(env.contract.address.to_string()),
        ))
}

/// Check if the sender is allowlisted for the hatch phase
fn assert_allowlisted(
    querier: QuerierWrapper,
    storage: &dyn Storage,
    hatcher: &Addr,
    hatch_config: &HatchConfig,
) -> Result<HatchConfig, ContractError> {
    if !hatcher_allowlist().is_empty(storage) {
        // Specific configs should trump everything
        if hatcher_allowlist().has(storage, hatcher) {
            let config = hatcher_allowlist().load(storage, hatcher)?;

            // Do not allow DAO's to purchase themselves when allowlisted as a DAO
            if matches!(
                config.config_type,
                HatcherAllowlistConfigType::DAO { priority: _ }
            ) {
                return Err(ContractError::SenderNotAllowlisted {
                    sender: hatcher.to_string(),
                });
            }

            return Ok(HatchConfig {
                contribution_limits: config
                    .contribution_limits_override
                    .unwrap_or(hatch_config.contribution_limits),
                ..*hatch_config
            });
        }

        // If not allowlisted as individual, then check any DAO allowlists
        return Ok(HatchConfig {
            contribution_limits: assert_allowlisted_through_daos(querier, storage, hatcher)?
                .unwrap_or(hatch_config.contribution_limits),
            ..*hatch_config
        });
    }

    Ok(*hatch_config)
}

fn assert_allowlisted_through_daos(
    querier: QuerierWrapper,
    storage: &dyn Storage,
    hatcher: &Addr,
) -> Result<Option<MinMax>, ContractError> {
    if let Some(hatcher_dao_priority_queue) = HATCHER_DAO_PRIORITY_QUEUE.may_load(storage)? {
        for entry in hatcher_dao_priority_queue {
            let voting_power_response_result: StdResult<
                dao_interface::voting::VotingPowerAtHeightResponse,
            > = querier.query_wasm_smart(
                entry.addr,
                &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                    address: hatcher.to_string(),
                    height: Some(entry.config.config_height),
                },
            );

            if let Ok(voting_power_response) = voting_power_response_result {
                if voting_power_response.power > Uint128::zero() {
                    return Ok(entry.config.contribution_limits_override);
                }
            }
        }
    }

    Err(ContractError::SenderNotAllowlisted {
        sender: hatcher.to_string(),
    })
}

/// Set the maximum supply (only callable by owner)
/// If `max_supply` is set to None there will be no limit.`
pub fn update_max_supply(
    deps: DepsMut,
    info: MessageInfo,
    max_supply: Option<Uint128>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    match max_supply {
        Some(max) => MAX_SUPPLY.save(deps.storage, &max)?,
        None => MAX_SUPPLY.remove(deps.storage),
    }

    Ok(Response::new()
        .add_attribute("action", "update_max_supply")
        .add_attribute("value", max_supply.unwrap_or(Uint128::MAX).to_string()))
}

/// Toggles the paused state (only callable by owner)
pub fn toggle_pause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let is_paused =
        IS_PAUSED.update(deps.storage, |is_paused| -> StdResult<_> { Ok(!is_paused) })?;

    Ok(Response::new()
        .add_attribute("action", "toggle_pause")
        .add_attribute("is_paused", is_paused.to_string()))
}

/// Add and remove addresses from the hatcher allowlist (only callable by owner and self)
pub fn update_hatch_allowlist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_add: Vec<HatcherAllowlistEntryMsg>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
    if env.contract.address != info.sender {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;
    }

    let list = hatcher_allowlist();

    // Add addresses to the allowlist
    for allow in to_add {
        let entry = allow.into_entry(deps.as_ref(), env.block.height)?;

        let old_data = list.may_load(deps.storage, &entry.addr)?;

        list.replace(
            deps.storage,
            &entry.addr,
            Some(&entry.config),
            old_data.as_ref(),
        )?;

        // If the old data was previously a DAO config, then it should be removed
        if let Some(old_data) = old_data {
            try_remove_from_priority_queue(deps.storage, &entry.addr, &old_data)?;
        }

        match allow.config.config_type {
            HatcherAllowlistConfigType::DAO { priority } => {
                if !HATCHER_DAO_PRIORITY_QUEUE.exists(deps.storage) {
                    HATCHER_DAO_PRIORITY_QUEUE.save(deps.storage, &vec![entry])?;
                } else {
                    HATCHER_DAO_PRIORITY_QUEUE.update(
                        deps.storage,
                        |mut queue| -> StdResult<_> {
                            match priority {
                                Some(priority_value) => {
                                    // Insert based on priority
                                    let pos = queue
                                        .binary_search_by(|entry| {
                                            match &entry.config.config_type {
                                                HatcherAllowlistConfigType::DAO {
                                                    priority: Some(entry_priority),
                                                } => entry_priority
                                                    .cmp(&priority_value)
                                                    .then(std::cmp::Ordering::Less),
                                                _ => std::cmp::Ordering::Less, // Treat non-DAO or DAO without priority as lower priority
                                            }
                                        })
                                        .unwrap_or_else(|e| e);
                                    queue.insert(pos, entry);
                                }
                                None => {
                                    // Append to the end if no priority
                                    queue.push(entry);
                                }
                            }

                            Ok(queue)
                        },
                    )?;
                }
            }
            HatcherAllowlistConfigType::Address {} => {}
        }
    }

    // Remove addresses from the allowlist
    for deny in to_remove {
        let addr = deps.api.addr_validate(deny.as_str())?;

        let old_data = list.may_load(deps.storage, &addr)?;

        if let Some(old_data) = old_data {
            list.replace(deps.storage, &addr, None, Some(&old_data))?;

            try_remove_from_priority_queue(deps.storage, &addr, &old_data)?;
        }
    }

    Ok(Response::new().add_attributes(vec![("action", "update_hatch_allowlist")]))
}

fn try_remove_from_priority_queue(
    storage: &mut dyn Storage,
    addr: &Addr,
    config: &HatcherAllowlistConfig,
) -> Result<(), ContractError> {
    if matches!(
        config.config_type,
        HatcherAllowlistConfigType::DAO { priority: _ }
    ) && HATCHER_DAO_PRIORITY_QUEUE.exists(storage)
    {
        HATCHER_DAO_PRIORITY_QUEUE.update(storage, |mut x| -> StdResult<_> {
            if let Some(i) = x.iter().position(|y| y.addr == addr) {
                x.remove(i);
            }

            Ok(x)
        })?;
    }

    Ok(())
}

/// Update the configuration of a particular phase (only callable by owner)
pub fn update_phase_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    update_phase_config_msg: UpdatePhaseConfigMsg,
) -> Result<Response, ContractError> {
    // Assert that the sender is the contract owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Load phase and phase config
    let phase = PHASE.load(deps.storage)?;

    // Load the current phase config
    let mut phase_config = PHASE_CONFIG.load(deps.storage)?;

    match update_phase_config_msg {
        UpdatePhaseConfigMsg::Hatch {
            initial_raise,
            entry_fee,
            contribution_limits,
        } => {
            // Check we are in the hatch phase
            phase.expect_hatch()?;

            // Update the hatch config if new values are provided
            if let Some(contribution_limits) = contribution_limits {
                phase_config.hatch.contribution_limits = contribution_limits;
            }
            if let Some(initial_raise) = initial_raise {
                phase_config.hatch.initial_raise = initial_raise;
            }
            if let Some(entry_fee) = entry_fee {
                phase_config.hatch.entry_fee = entry_fee;
            }

            // Validate config
            phase_config.hatch.validate()?;
            PHASE_CONFIG.save(deps.storage, &phase_config)?;

            Ok(Response::new().add_attribute("action", "update_hatch_phase_config"))
        }
        UpdatePhaseConfigMsg::Open {
            exit_fee,
            entry_fee,
        } => {
            // Check we are in the open phase
            phase.expect_open()?;

            // Update the hatch config if new values are provided
            if let Some(entry_fee) = entry_fee {
                phase_config.open.entry_fee = entry_fee;
            }
            if let Some(exit_fee) = exit_fee {
                phase_config.open.exit_fee = exit_fee;
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

/// Update the bonding curve. (only callable by owner)
/// NOTE: this changes the pricing. Use with caution.
/// TODO: what other limitations do we want to put on this?
pub fn update_curve(
    deps: DepsMut,
    info: MessageInfo,
    curve_type: CurveType,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    CURVE_TYPE.save(deps.storage, &curve_type)?;

    Ok(Response::new().add_attribute("action", "close"))
}

/// Update the ownership of the contract
pub fn update_ownership(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
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
        use crate::testing::{mock_init, TEST_CREATOR};
        use cosmwasm_std::coin;
        use cw_utils::PaymentError;

        const TEST_DONOR: &str = "donor";

        fn exec_donate(deps: DepsMut, donation_amount: u128) -> Result<Response, ContractError> {
            donate(
                deps,
                mock_env(),
                mock_info(TEST_DONOR, &[coin(donation_amount, TEST_RESERVE_DENOM)]),
            )
        }

        #[test]
        fn should_fail_with_no_funds() -> Result<(), ContractError> {
            let mut deps = mock_dependencies();
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
        fn should_fail_with_incorrect_denom() -> Result<(), ContractError> {
            let mut deps = mock_dependencies();
            let curve_type = CurveType::Linear {
                slope: Uint128::new(1),
                scale: 1,
            };
            let init_msg = default_instantiate_msg(2, 8, curve_type);
            mock_init(deps.as_mut(), init_msg)?;

            let res = donate(
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
        fn should_donate_to_forwarding() -> Result<(), ContractError> {
            let mut deps = mock_dependencies();
            // this matches `linear_curve` test case from curves.rs
            let curve_type = CurveType::SquareRoot {
                slope: Uint128::new(1),
                scale: 1,
            };
            let mut init_msg = default_instantiate_msg(2, 8, curve_type);
            init_msg.funding_pool_forwarding = Some(TEST_CREATOR.to_string());
            mock_init(deps.as_mut(), init_msg)?;

            let donation_amount = 5;
            let _res = exec_donate(deps.as_mut(), donation_amount)?;

            // Check that the funding pool did not increase, because it was sent to the funding pool forwarding
            // NOTE: the balance cannot be checked with mock_dependencies
            let curve_state = CURVE_STATE.load(&deps.storage)?;
            assert_that!(curve_state.funding).is_equal_to(Uint128::zero());

            // check that the donor is in the donations map
            let donation = DONATIONS.load(&deps.storage, &Addr::unchecked(TEST_DONOR))?;
            assert_that!(donation).is_equal_to(Uint128::new(donation_amount));

            Ok(())
        }

        #[test]
        fn test_donate_and_withdraw() -> Result<(), ContractError> {
            // Init
            let mut deps = mock_dependencies();

            let curve_type = CurveType::SquareRoot {
                slope: Uint128::new(1),
                scale: 1,
            };
            let init_msg = default_instantiate_msg(2, 8, curve_type);
            mock_init(deps.as_mut(), init_msg)?;

            // Donate
            let donation_amount = 5;
            let _res = exec_donate(deps.as_mut(), donation_amount)?;

            // Check funding pool
            let curve_state = CURVE_STATE.load(&deps.storage)?;
            assert_that!(curve_state.funding).is_equal_to(Uint128::from(donation_amount));

            // Check random can't withdraw from the funding pool
            let result = withdraw(deps.as_mut(), mock_env(), mock_info("random", &[]), None);
            assert_that!(result)
                .is_err()
                .is_equal_to(ContractError::Ownership(
                    cw_ownable::OwnershipError::NotOwner,
                ));

            // Check owner can withdraw
            let result = withdraw(
                deps.as_mut(),
                mock_env(),
                mock_info(crate::testing::TEST_CREATOR, &[]),
                None,
            );
            assert!(result.is_ok());

            Ok(())
        }

        #[test]
        fn test_pause() -> Result<(), ContractError> {
            let mut deps = mock_dependencies();
            // this matches `linear_curve` test case from curves.rs
            let curve_type = CurveType::SquareRoot {
                slope: Uint128::new(1),
                scale: 1,
            };
            let init_msg = default_instantiate_msg(2, 8, curve_type);
            mock_init(deps.as_mut(), init_msg)?;

            // Ensure not paused on instantiate
            assert!(!IS_PAUSED.load(&deps.storage)?);

            // Ensure random cannot pause
            let res = toggle_pause(deps.as_mut(), mock_info("random", &[]));
            assert_that!(res)
                .is_err()
                .is_equal_to(ContractError::Ownership(
                    cw_ownable::OwnershipError::NotOwner,
                ));

            // Ensure paused after toggling
            toggle_pause(deps.as_mut(), mock_info(TEST_CREATOR, &[]))?;
            assert!(IS_PAUSED.load(&deps.storage)?);

            // Ensure random cannot do anything
            let res = crate::contract::execute(
                deps.as_mut(),
                mock_env(),
                mock_info("random", &[]),
                crate::msg::ExecuteMsg::TogglePause {},
            );
            assert_that!(res)
                .is_err()
                .is_equal_to(ContractError::Paused {});

            // Ensure unpaused after toggling
            toggle_pause(deps.as_mut(), mock_info(TEST_CREATOR, &[]))?;
            assert!(!IS_PAUSED.load(&deps.storage)?);

            Ok(())
        }
    }
}
