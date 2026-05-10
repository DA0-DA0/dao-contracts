use cosmwasm_std::{
    to_json_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, Response, StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw_tokenfactory_issuer::msg::ExecuteMsg as IssuerExecuteMsg;
use cw_utils::must_pay;
use std::ops::Deref;

use crate::abc::{CommonsPhase, CurveType, HatchConfig, MinMax};
use crate::helpers::{calculate_buy_quote, calculate_sell_quote, vested_amount};
use crate::msg::{HatcherAllowlistEntryMsg, UpdatePhaseConfigMsg};
use crate::state::{
    hatcher_allowlist, HatcherAllowlistConfig, HatcherAllowlistConfigType, RefundSnapshot,
    CURVE_STATE, CURVE_TYPE, DONATIONS, FUNDING_POOL_FORWARDING, HATCHERS,
    HATCHER_DAO_PRIORITY_QUEUE, IS_PAUSED, MAX_SUPPLY, PHASE, PHASE_CONFIG, REFUND_SNAPSHOT,
    SUPPLY_DENOM, TOKEN_ISSUER_CONTRACT,
};
use crate::ContractError;

pub fn buy(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    let payment = must_pay(&info, &curve_state.reserve_denom)?;

    // Load the phase config and phase
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let mut phase = PHASE.load(deps.storage)?;

    // Calculate the curve state from the buy
    let buy_quote = calculate_buy_quote(payment, &curve_type, &curve_state, &phase, &phase_config)?;

    // L-2: collect any allowlist-warning attributes for surfacing on the
    // buy response (e.g. `try_dao_query_failed` events when a configured
    // DAO entry's voting-power query errored).
    let mut allowlist_attrs: Vec<Attribute> = vec![];

    // Validate phase
    match &phase {
        CommonsPhase::Hatch => {
            // Check that the potential hatcher is allowlisted
            let (hatch_config, attrs) = assert_allowlisted(
                deps.querier,
                deps.storage,
                &info.sender,
                &phase_config.hatch,
            )?;
            allowlist_attrs = attrs;

            // Update hatcher state with the gross contribution and the
            // freshly-minted tokens.
            let updated_state =
                HATCHERS.update(deps.storage, &info.sender, |maybe| -> StdResult<_> {
                    let mut state = maybe.unwrap_or_default();
                    state.contributed = state.contributed.checked_add(payment)?;
                    state.minted = state.minted.checked_add(buy_quote.amount)?;
                    Ok(state)
                })?;

            // Check contribution is within limits (uses gross contribution)
            if updated_state.contributed < hatch_config.contribution_limits.min
                || updated_state.contributed > hatch_config.contribution_limits.max
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

                // Stamp every hatcher's vesting_started_at so their tokens
                // begin unlocking from the transition time. Iteration is
                // O(N hatchers); N is bounded by initial_raise.max divided
                // by contribution_limits.min.
                let now = env.block.time;
                stamp_hatcher_vesting_clocks(deps.storage, now)?;

                // Allowlist no longer needed
                hatcher_allowlist().clear(deps.storage);

                PHASE.save(deps.storage, &phase)?;
            }
        }
        CommonsPhase::Open => {}
        CommonsPhase::Closed => {
            return Err(ContractError::CommonsClosed {});
        }
        CommonsPhase::Refunding => {
            return Err(ContractError::CommonsClosed {});
        }
    };

    // Check that the minted amount has not exceeded the max supply (if configured).
    // I-6: strict `>` is intentional — a buy that brings new_supply to exactly
    // max_supply is allowed; the next buy will be rejected.
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
        .add_attribute("supply", buy_quote.new_supply)
        .add_attributes(allowlist_attrs))
}

/// Stamp `vesting_started_at` on every existing hatcher entry. Called once
/// at the Hatch → Open transition. Hatchers added after this point (i.e.
/// no one — we forbid hatch buys after Open) would not need stamping.
fn stamp_hatcher_vesting_clocks(
    storage: &mut dyn Storage,
    now: Timestamp,
) -> Result<(), ContractError> {
    let addrs: Vec<Addr> = HATCHERS
        .keys(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for addr in addrs {
        HATCHERS.update(storage, &addr, |maybe| -> StdResult<_> {
            let mut state = maybe.unwrap_or_default();
            state.vesting_started_at = Some(now);
            Ok(state)
        })?;
    }
    Ok(())
}

/// Sell tokens on the bonding curve
pub fn sell(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let supply_denom = SUPPLY_DENOM.load(deps.storage)?;
    let burn_amount = must_pay(&info, &supply_denom)?;

    let mut curve_state = CURVE_STATE.load(deps.storage)?;

    // Load the phase configuration and the current phase
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let phase = PHASE.load(deps.storage)?;

    // H-1: enforce hatcher vesting before letting hatchers exit.
    // Non-hatchers (no entry in HATCHERS) sell freely. Hatchers' tokens
    // unlock per `phase_config.vesting`; sells consume from the unlocked
    // portion via `state.already_burned`.
    if let Some(state) = HATCHERS.may_load(deps.storage, &info.sender)? {
        let vested = vested_amount(&state, &phase_config.vesting, env.block.time);
        let available = vested.saturating_sub(state.already_burned);
        if burn_amount > available {
            return Err(ContractError::HatcherTokensNotVested {
                requested: burn_amount,
                available,
            });
        }
        let mut updated = state;
        updated.already_burned = updated.already_burned.checked_add(burn_amount)?;
        HATCHERS.save(deps.storage, &info.sender, &updated)?;
    }

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

    // M-5 full: Refunding is terminal (driven by AbortHatch); cannot be
    // overridden by Close.
    let phase = PHASE.load(deps.storage)?;
    if matches!(phase, CommonsPhase::Refunding) {
        return Err(ContractError::InvalidPhase {
            expected: "Hatch | Open".to_string(),
            actual: "Refunding".to_string(),
        });
    }

    // I-2: zero out the open-phase exit fee so the stored config matches
    // runtime behavior (calculate_sell_quote returns Decimal::zero() for
    // Closed phase regardless, but indexers / UIs read the config and
    // were previously misled by a non-zero exit_fee post-close).
    let mut phase_config = PHASE_CONFIG.load(deps.storage)?;
    phase_config.open.exit_fee = cosmwasm_std::Decimal::zero();
    PHASE_CONFIG.save(deps.storage, &phase_config)?;

    PHASE.save(deps.storage, &CommonsPhase::Closed)?;

    Ok(Response::new().add_attribute("action", "close"))
}

/// Abort a stalled hatch. Callable by anyone after `hatch_deadline` has
/// passed if the curve has not reached `initial_raise.min`. Snapshots the
/// pro-rata refund math and transitions to `CommonsPhase::Refunding`, where
/// hatchers claim via `ClaimRefund`. Closes audit M-5 (full).
pub fn abort_hatch(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let phase = PHASE.load(deps.storage)?;
    phase.expect_hatch()?;

    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let deadline =
        phase_config
            .hatch
            .hatch_deadline
            .ok_or(ContractError::HatchPhaseConfigError(
                "No hatch_deadline configured; abort not allowed".to_string(),
            ))?;
    if env.block.time < deadline {
        return Err(ContractError::HatchPhaseConfigError(format!(
            "Hatch deadline (epoch {}) not yet reached",
            deadline.seconds()
        )));
    }

    let curve_state = CURVE_STATE.load(deps.storage)?;
    if curve_state.reserve >= phase_config.hatch.initial_raise.min {
        return Err(ContractError::HatchPhaseConfigError(format!(
            "Hatch reached the minimum raise ({}); abort not allowed",
            phase_config.hatch.initial_raise.min
        )));
    }

    // Snapshot the pool and total contributions so claims are deterministic.
    let total_pool = curve_state.reserve.checked_add(curve_state.funding)?;
    let total_contributed = HATCHERS
        .range(deps.storage, None, None, Order::Ascending)
        .try_fold(Uint128::zero(), |acc, item| -> StdResult<_> {
            let (_addr, state) = item?;
            Ok(acc + state.contributed)
        })?;

    REFUND_SNAPSHOT.save(
        deps.storage,
        &RefundSnapshot {
            total_pool,
            total_contributed,
        },
    )?;

    PHASE.save(deps.storage, &CommonsPhase::Refunding)?;

    Ok(Response::new()
        .add_attribute("action", "abort_hatch")
        .add_attribute("total_pool", total_pool)
        .add_attribute("total_contributed", total_contributed))
}

/// Claim a hatcher's pro-rata refund during the Refunding phase. Caller
/// must be a hatcher who has not yet claimed; must send their unburned
/// hatcher tokens for burning. Returns `state.contributed * snapshot.total_pool
/// / snapshot.total_contributed` reserve tokens.
pub fn claim_refund(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let phase = PHASE.load(deps.storage)?;
    phase.expect_refunding()?;

    let mut state = HATCHERS.may_load(deps.storage, &info.sender)?.ok_or(
        ContractError::SenderNotAllowlisted {
            sender: info.sender.to_string(),
        },
    )?;

    if state.claimed_refund {
        return Err(ContractError::RefundAlreadyClaimed {});
    }

    let snapshot = REFUND_SNAPSHOT.load(deps.storage)?;
    if snapshot.total_contributed.is_zero() {
        return Err(ContractError::HatchPhaseConfigError(
            "No refund snapshot — nothing to claim".to_string(),
        ));
    }

    // Hatcher must surrender their unburned hatcher tokens.
    let supply_denom = SUPPLY_DENOM.load(deps.storage)?;
    let burn_amount = must_pay(&info, &supply_denom)?;
    let unburned = state.minted.checked_sub(state.already_burned)?;
    if burn_amount != unburned {
        return Err(ContractError::RefundBurnMismatch {
            expected: unburned,
            sent: burn_amount,
        });
    }

    // Pro-rata refund.
    let refund = state
        .contributed
        .multiply_ratio(snapshot.total_pool, snapshot.total_contributed);

    // Mark claimed first so re-entrancy via reply can't double-claim.
    state.already_burned = state.minted;
    state.claimed_refund = true;
    HATCHERS.save(deps.storage, &info.sender, &state)?;

    let curve_state = CURVE_STATE.load(deps.storage)?;
    let issuer_addr = TOKEN_ISSUER_CONTRACT.load(deps.storage)?;

    let mut msgs: Vec<CosmosMsg> = vec![
        // Send hatcher tokens to issuer for burning
        CosmosMsg::Bank(BankMsg::Send {
            to_address: issuer_addr.to_string(),
            amount: vec![Coin {
                amount: burn_amount,
                denom: supply_denom,
            }],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: issuer_addr.to_string(),
            msg: to_json_binary(&IssuerExecuteMsg::Burn {
                from_address: issuer_addr.to_string(),
                amount: burn_amount,
            })?,
            funds: vec![],
        }),
    ];

    // Send pro-rata refund.
    if !refund.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                amount: refund,
                denom: curve_state.reserve_denom,
            }],
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim_refund")
        .add_attribute("hatcher", info.sender)
        .add_attribute("contributed", state.contributed)
        .add_attribute("refund", refund)
        .add_attribute("burned", burn_amount))
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

    // M-5 full: in Refunding the funding pool belongs to hatchers via
    // pro-rata claims; owner cannot drain it.
    let phase = PHASE.load(deps.storage)?;
    if matches!(phase, CommonsPhase::Refunding) {
        return Err(ContractError::InvalidPhase {
            expected: "Hatch | Open | Closed".to_string(),
            actual: "Refunding".to_string(),
        });
    }

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

/// Check if the sender is allowlisted for the hatch phase. Returns the
/// effective HatchConfig along with any debug attributes (e.g.
/// `try_dao_query_failed` events) that callers should attach to their
/// response so operators can monitor allowlist health.
fn assert_allowlisted(
    querier: QuerierWrapper,
    storage: &dyn Storage,
    hatcher: &Addr,
    hatch_config: &HatchConfig,
) -> Result<(HatchConfig, Vec<Attribute>), ContractError> {
    if !hatcher_allowlist().is_empty(storage) {
        // Specific configs should trump everything
        if hatcher_allowlist().has(storage, hatcher) {
            let config = hatcher_allowlist().load(storage, hatcher)?;

            // I-5: per-address entries that carry the DAO config type are
            // intended to be allowlisted DAOs themselves, not individual
            // hatchers — the DAO does not get its own voting power so it
            // wouldn't pass the through-DAO path either, and we want a
            // clear rejection rather than silent fall-through. Side
            // effect: a regular individual added with type DAO is locked
            // out; operators should add individuals as type Address.
            if matches!(
                config.config_type,
                HatcherAllowlistConfigType::DAO { priority: _ }
            ) {
                return Err(ContractError::SenderNotAllowlisted {
                    sender: hatcher.to_string(),
                });
            }

            return Ok((
                HatchConfig {
                    contribution_limits: config
                        .contribution_limits_override
                        .unwrap_or(hatch_config.contribution_limits),
                    ..*hatch_config
                },
                vec![],
            ));
        }

        // If not allowlisted as individual, then check any DAO allowlists.
        let (override_limits, attrs) =
            assert_allowlisted_through_daos(querier, storage, hatcher)?;
        return Ok((
            HatchConfig {
                contribution_limits: override_limits.unwrap_or(hatch_config.contribution_limits),
                ..*hatch_config
            },
            attrs,
        ));
    }

    Ok((*hatch_config, vec![]))
}

/// Iterate the priority queue of DAO allowlist entries, returning the first
/// `contribution_limits_override` whose DAO grants the hatcher voting power.
///
/// L-2: returns the accumulated `try_dao_query_failed` attributes alongside
/// the result so callers can surface them on the response. The entries that
/// errored (rather than returned zero power) are surfaced this way so
/// operators can detect a misconfigured / migrated DAO without inferring
/// from gas profile alone.
fn assert_allowlisted_through_daos(
    querier: QuerierWrapper,
    storage: &dyn Storage,
    hatcher: &Addr,
) -> Result<(Option<MinMax>, Vec<Attribute>), ContractError> {
    let mut attrs: Vec<Attribute> = vec![];
    if let Some(hatcher_dao_priority_queue) = HATCHER_DAO_PRIORITY_QUEUE.may_load(storage)? {
        for entry in hatcher_dao_priority_queue {
            let voting_power_response_result: StdResult<
                dao_interface::voting::VotingPowerAtHeightResponse,
            > = querier.query_wasm_smart(
                entry.addr.clone(),
                &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                    address: hatcher.to_string(),
                    height: Some(entry.config.config_height),
                },
            );

            match voting_power_response_result {
                Ok(voting_power_response) => {
                    if voting_power_response.power > Uint128::zero() {
                        return Ok((entry.config.contribution_limits_override, attrs));
                    }
                }
                Err(_e) => {
                    // L-2: surface the failed-query DAO on the response so
                    // operators can detect a stale / misconfigured entry.
                    // We continue to the next entry rather than aborting
                    // — a single broken DAO shouldn't lock out users who
                    // have voting power in a different allowlisted DAO.
                    attrs.push(Attribute::new(
                        "try_dao_query_failed",
                        entry.addr.to_string(),
                    ));
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

/// Add and remove addresses from the hatcher allowlist (only callable by
/// owner). The instantiate path calls this directly rather than via a
/// self-message so there is no auth-bypass branch (audit fix L-3).
pub fn update_hatch_allowlist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_add: Vec<HatcherAllowlistEntryMsg>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let _ = env; // env was previously needed for self-call check; kept in signature for now

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
                            insert_into_priority_queue(&mut queue, entry, priority);
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

/// Insert a new DAO-typed allowlist entry into the priority queue while
/// keeping the queue sorted: `Some(priority)` entries first in ascending
/// priority-value order, `None`-priority entries appended at the end. Uses
/// `partition_point`, which assumes the queue is already sorted in this
/// shape — true by induction from the empty case if all inserts go through
/// this function. Closes audit finding M-1 (the previous binary_search_by
/// approach was a no-op due to a comparator that never returned Equal).
pub(crate) fn insert_into_priority_queue(
    queue: &mut Vec<crate::state::HatcherAllowlistEntry>,
    entry: crate::state::HatcherAllowlistEntry,
    priority: Option<cosmwasm_std::Uint64>,
) {
    match priority {
        Some(priority_value) => {
            let pos = queue.partition_point(|existing| {
                match &existing.config.config_type {
                    HatcherAllowlistConfigType::DAO { priority: Some(p) } => *p <= priority_value,
                    // None-priority entries sort after all Some entries.
                    HatcherAllowlistConfigType::DAO { priority: None } => false,
                    HatcherAllowlistConfigType::Address {} => false,
                }
            });
            queue.insert(pos, entry);
        }
        None => {
            queue.push(entry);
        }
    }
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
            hatch_deadline,
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
            // Some(None) clears the deadline; Some(Some(t)) sets it; None
            // leaves the existing value untouched.
            if let Some(hatch_deadline) = hatch_deadline {
                phase_config.hatch.hatch_deadline = hatch_deadline;
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
    }
}

/// Update the bonding curve. Only callable by the owner, and only while the
/// commons is in the Closed phase. The new curve must produce a reserve at
/// the existing supply that is within `MAX_CURVE_DRIFT` of the recorded
/// reserve, so that an invariant break (mint-flood / sell-deadlock) is not
/// possible via curve replacement.
pub fn update_curve(
    deps: DepsMut,
    info: MessageInfo,
    curve_type: CurveType,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let phase = PHASE.load(deps.storage)?;
    phase.expect_closed()?;

    let curve_state = CURVE_STATE.load(deps.storage)?;
    let new_curve = curve_type.to_curve_fn()(curve_state.decimals);
    let new_reserve_at_supply = new_curve.reserve(curve_state.supply)?;

    // Reject any curve that would not honor the existing (reserve, supply)
    // pair within tolerance. Tolerance is for floor-rounding accumulation,
    // not for arbitrary swaps.
    let drift = if new_reserve_at_supply > curve_state.reserve {
        new_reserve_at_supply - curve_state.reserve
    } else {
        curve_state.reserve - new_reserve_at_supply
    };
    let tolerance = curve_state
        .reserve
        .multiply_ratio(MAX_CURVE_DRIFT_BPS, 10_000u128);
    if drift > tolerance {
        return Err(ContractError::CurveDriftExceeded {
            current_reserve: curve_state.reserve,
            new_reserve_at_current_supply: new_reserve_at_supply,
            tolerance,
        });
    }

    CURVE_TYPE.save(deps.storage, &curve_type)?;

    Ok(Response::new().add_attribute("action", "update_curve"))
}

/// Maximum allowed drift between the existing reserve and what a replacement
/// curve would imply at the current supply, expressed in basis points
/// (1 bp = 0.01%). 100 bps = 1% — wide enough to absorb floor-rounding
/// across curves with different precision shapes, narrow enough to prevent
/// rug via curve swap.
const MAX_CURVE_DRIFT_BPS: u128 = 100;

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
