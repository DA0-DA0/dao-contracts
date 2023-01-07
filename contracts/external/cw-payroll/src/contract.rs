#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, Order, Response, StakingMsg, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;
use cw_paginate::paginate_map_values;
use cw_utils::must_pay;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{
    UncheckedVestingParams, ValidatorRewards, VestingPayment, VestingPaymentStatus,
    STAKED_VESTING_CLAIMS, VALIDATORS_REWARDS, VESTING_PAYMENTS, VESTING_PAYMENT_SEQ,
};

const CONTRACT_NAME: &str = "crates.io:cw-payroll";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    VESTING_PAYMENT_SEQ.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string())))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive_cw20(env, deps, info, msg),
        ExecuteMsg::Create(vesting_params) => {
            execute_create_vesting_payment_native(env, deps, info, vesting_params)
        }
        ExecuteMsg::Distribute { id } => execute_distribute(env, deps, id),
        ExecuteMsg::Cancel { id } => execute_cancel_vesting_payment(env, deps, info, id),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::Delegate {
            vesting_payment_id,
            validator,
            amount,
        } => execute_delegate(env, deps, info, vesting_payment_id, validator, amount),
        ExecuteMsg::Undelegate {
            vesting_payment_id,
            validator,
            amount,
        } => execute_undelegate(env, deps, info, vesting_payment_id, validator, amount),
        ExecuteMsg::WithdrawDelegatorReward { validator } => {
            execute_withdraw_rewards(env, deps, info, validator)
        }
    }
}

pub fn execute_create_vesting_payment_native(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    unchecked_vesting_params: UncheckedVestingParams,
) -> Result<Response, ContractError> {
    // Validate all vesting payment params
    let checked_params = unchecked_vesting_params.into_checked(deps.as_ref())?;

    // Check amount sent matches amount in vesting payment
    if checked_params.amount != must_pay(&info, &checked_params.denom.to_string())? {
        return Err(ContractError::AmountDoesNotMatch);
    }

    // Create a new vesting payment
    let vesting_payment = VestingPayment::new(deps, checked_params)?;

    Ok(Response::new()
        .add_attribute("method", "create_vesting_payment")
        .add_attribute("vesting_payment_id", vesting_payment.id.to_string())
        .add_attribute("recipient", vesting_payment.recipient.to_string()))
}

pub fn execute_receive_cw20(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&receive_msg.msg)?;

    match msg {
        ReceiveMsg::Create(msg) => {
            // Validate all vesting schedule, recipient address, and denom
            let checked_params = msg.into_checked(deps.as_ref())?;

            // Check amount sent matches amount in vesting payment
            if checked_params.amount != receive_msg.amount {
                return Err(ContractError::AmountDoesNotMatch);
            }

            // Check that the Cw20 specified in the denom *must be* matches sender
            if info.sender.to_string() != checked_params.denom.to_string() {
                return Err(ContractError::Cw20DoesNotMatch);
            }

            // Create a new vesting payment
            let vesting_payment = VestingPayment::new(deps, checked_params)?;

            Ok(Response::new()
                .add_attribute("method", "create_vesting_payment")
                .add_attribute("vesting_payment_id", vesting_payment.id.to_string())
                .add_attribute("recipient", vesting_payment.recipient.to_string()))
        }
    }
}

pub fn execute_cancel_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    // Check sender is contract owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let vesting_payment = VESTING_PAYMENTS.update(deps.storage, id, |vp| match vp {
        Some(mut vp) => {
            vp.status = VestingPaymentStatus::Canceled;
            Ok(vp)
        }
        None => Err(ContractError::VestingPaymentNotFound {
            vesting_payment_id: id,
        }),
    })?;

    // TODO unbond any staked funds...
    // TODO get all validators
    // let unbond_msg = StakingMsg::Delegate {
    //     validator: validator.clone(),
    //     amount: Coin { denom, amount },
    // };

    // Check if funds have vested
    let vested_amount = vesting_payment.get_vested_amount_by_seconds(env.block.time.seconds())?;

    // TODO handle edge case where funds are staked...
    // Transfer any remaining unvested amount to the owner
    let transfer_unvested_to_owner_msg = vesting_payment
        .denom
        .get_transfer_to_message(&info.sender, vesting_payment.amount - vested_amount)?;

    let transfer_vested_to_recipient_msg = vesting_payment
        .denom
        .get_transfer_to_message(&vesting_payment.recipient, vested_amount)?;

    Ok(Response::new()
        .add_attribute("method", "remove_vesting_payment")
        .add_attribute("vesting_payment_id", id.to_string())
        .add_attribute("owner", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_message(transfer_unvested_to_owner_msg)
        .add_message(transfer_vested_to_recipient_msg))
}

pub fn execute_distribute(env: Env, deps: DepsMut, id: u64) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENTS.may_load(deps.storage, id)?.ok_or(
        ContractError::VestingPaymentNotFound {
            vesting_payment_id: id,
        },
    )?;

    let vested_amount = vesting_payment.get_vested_amount_by_seconds(env.block.time.seconds())?;
    let staking_rewards = vesting_payment.pending_to_u128()?;

    if vested_amount == Uint128::zero() && staking_rewards == 0 {
        return Err(ContractError::NoFundsToClaim {
            claimed: vesting_payment.claimed_amount,
        });
    }

    // Update Vesting Payment with claimed amount
    VESTING_PAYMENTS.update(deps.storage, id, |v| -> Result<_, ContractError> {
        match v {
            Some(mut v) => {
                // Update amounts
                v.amount -= vested_amount;
                v.claimed_amount += vested_amount;
                v.rewards.pending = Decimal::zero();
                // If the amount remaining in contract goes to zero, update status
                if v.amount == Uint128::zero() {
                    v.status = VestingPaymentStatus::FullyVested
                }
                Ok(v)
            }
            None => Err(ContractError::VestingPaymentNotFound {
                vesting_payment_id: id,
            }),
        }
    })?;

    // Get transfer message for payout
    let total_payout = vested_amount + Uint128::new(staking_rewards);
    let transfer_msg = vesting_payment
        .denom
        .get_transfer_to_message(&vesting_payment.recipient, total_payout)?;

    // TODO handle edge case where contract has been canceled while funds are staked
    if vesting_payment.status == VestingPaymentStatus::Canceled {
        return Err(ContractError::VestingPaymentCanceled {
            vesting_payment_id: id,
        });
    }

    Ok(Response::new()
        .add_attribute("method", "distribute")
        .add_attribute("vested_amount", vested_amount)
        .add_attribute("vesting_payment_id", id.to_string())
        .add_message(transfer_msg))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

pub fn execute_delegate(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENTS.load(deps.storage, id)?;

    // Check status isn't canceled
    if vesting_payment.status == VestingPaymentStatus::Canceled {
        return Err(ContractError::VestingPaymentCanceled {
            vesting_payment_id: id,
        });
    }

    // Check sender is the recipient of the vesting payment
    if info.sender != vesting_payment.recipient {
        return Err(ContractError::Unauthorized);
    }

    // Check vesting payment denom matches local denom
    let denom = match vesting_payment.denom {
        CheckedDenom::Cw20(_) => {
            return Err(ContractError::NotStakeable);
        }
        CheckedDenom::Native(denom) => {
            // Get local denom, ensure it matches the vesting denom
            let local_denom = deps.querier.query_bonded_denom()?;
            if local_denom != denom {
                return Err(ContractError::NotStakeable);
            } else {
                denom
            }
        }
    };

    // Update vesting payment with amount of staked rewards
    VESTING_PAYMENTS.update(deps.storage, id, |v| -> Result<_, ContractError> {
        match v {
            Some(mut v) => {
                // Check stake amount is not greater than vesting payment amount
                if amount > v.amount {
                    return Err(ContractError::NotEnoughFunds {});
                }

                // Check that amount has not already been staked
                v.amount.checked_sub(v.staked_amount + amount)?;

                // Update amounts
                v.staked_amount += amount;
                Ok(v)
            }
            None => Err(ContractError::VestingPaymentNotFound {
                vesting_payment_id: id,
            }),
        }
    })?;

    // Save a record of staking this vesting payment amount to a validator
    let previously_staked = STAKED_VESTING_CLAIMS.may_load(deps.storage, (&validator, id))?;
    match previously_staked {
        Some(staked_amount) => {
            STAKED_VESTING_CLAIMS.save(deps.storage, (&validator, id), &(staked_amount + amount))
        }
        None => STAKED_VESTING_CLAIMS.save(deps.storage, (&validator, id), &amount),
    }?;

    // If its a first delegation to a validator, we set validator rewards to 0
    let validator_rewards = VALIDATORS_REWARDS.may_load(deps.storage, &validator)?;
    match validator_rewards {
        Some(val_rewards) => val_rewards,
        None => {
            let val = ValidatorRewards::default();

            VALIDATORS_REWARDS.save(deps.storage, &validator, &val)?;
            val
        }
    };

    // Create message to delegate the underlying tokens
    let msg = StakingMsg::Delegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    Ok(Response::new()
        .add_attribute("method", "delegate")
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

pub fn execute_undelegate(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENTS.load(deps.storage, id)?;

    // Check sender is the recipient of the vesting payment
    if info.sender != vesting_payment.recipient {
        return Err(ContractError::Unauthorized);
    }

    let delegations = STAKED_VESTING_CLAIMS
        .may_load(deps.storage, (&validator, id))?
        .ok_or(ContractError::NoDelegationsForValidator {})?;

    let validator_rewards = VALIDATORS_REWARDS.load(deps.storage, &validator)?;

    // Update vesting payment with amount of staked rewards
    VESTING_PAYMENTS.update(deps.storage, id, |v| -> Result<_, ContractError> {
        match v {
            Some(mut v) => {
                // Check unstake amount is not greater than vesting payment amount
                if amount > v.staked_amount {
                    // TODO better error message
                    return Err(ContractError::NotEnoughFunds {});
                }

                // Update pending rewards
                v.calc_pending_rewards(validator_rewards.rewards_per_token, delegations)?;

                // Update amounts
                v.staked_amount -= amount;
                Ok(v)
            }
            None => Err(ContractError::VestingPaymentNotFound {
                vesting_payment_id: id,
            }),
        }
    })?;

    let denom = deps.querier.query_bonded_denom()?;

    // Create message to delegate the underlying tokens
    let msg = StakingMsg::Undelegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    // Update amount delegated to validator
    STAKED_VESTING_CLAIMS.save(deps.storage, (&validator, id), &(delegations - amount))?;

    Ok(Response::default().add_message(msg))
}

pub fn execute_withdraw_rewards(
    env: Env,
    deps: DepsMut,
    _info: MessageInfo,
    validator: String,
) -> Result<Response, ContractError> {
    // Query fullDelegation to get the total rewards amount
    let delegation_query = deps
        .querier
        .query_delegation(env.contract.address, validator.clone())?;

    // Total rewards we have from this validator
    let total_accumulated_rewards = &match delegation_query {
        Some(delegation) => delegation.accumulated_rewards,
        None => return Err(ContractError::NoDelegationsForValidator {}),
    };

    let total_delegations = STAKED_VESTING_CLAIMS
        .prefix(&validator)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| -> StdResult<Uint128> { Ok(res?.1) })
        .sum::<StdResult<Uint128>>()?;

    // Check to make sure there are rewards
    if total_accumulated_rewards.is_empty() || total_accumulated_rewards[0].amount.is_zero() {
        return Err(ContractError::ZeroRewardsToSend {});
    }

    let total_accumulated_rewards = &total_accumulated_rewards[0];

    // Update rewards for this validator
    VALIDATORS_REWARDS.update(
        deps.storage,
        &validator,
        |rewards: Option<ValidatorRewards>| -> Result<_, ContractError> {
            let mut validator_rewards = rewards.unwrap();

            validator_rewards.calc_rewards(total_accumulated_rewards.amount, total_delegations)?;
            Ok(validator_rewards)
        },
    )?;

    // Withdraw rewards from validator
    let withdraw_msg =
        CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward { validator });

    Ok(Response::default().add_message(withdraw_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetVestingPayment { id } => to_binary(&VESTING_PAYMENTS.load(deps.storage, id)?),
        QueryMsg::ListVestingPayments { start_after, limit } => to_binary(&paginate_map_values(
            deps,
            &VESTING_PAYMENTS,
            start_after,
            limit,
            Order::Descending,
        )?),
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
