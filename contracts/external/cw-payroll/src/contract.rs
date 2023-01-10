#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, Order, Response, StakingMsg, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_utils::{must_pay, nonpayable};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{
    ValidatorRewards, VestingPayment, VestingPaymentStatus, STAKED_VESTING_BY_VALIDATOR,
    VALIDATORS_REWARDS, VESTING_PAYMENT,
};

const CONTRACT_NAME: &str = "crates.io:cw-payroll";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    // Check all params and create vesting payment with status based on denom
    let vp = match msg.params.denom {
        UncheckedDenom::Native(_) => {
            // Validate all vesting payment params
            let checked_params = msg.params.into_checked(deps.as_ref())?;

            // Check amount sent matches amount in vesting payment
            if checked_params.amount != must_pay(&info, &checked_params.denom.to_string())? {
                return Err(ContractError::AmountDoesNotMatch);
            }

            // Create a new vesting payment
            VestingPayment::new(checked_params)?
        }
        UncheckedDenom::Cw20(_) => {
            // If configured for cw20, check no native funds included
            nonpayable(&info)?;

            // Validate all vesting payment params
            let checked_params = msg.params.into_checked(deps.as_ref())?;

            // Create new vesting payment
            let mut vesting_payment = VestingPayment::new(checked_params)?;

            // Set status to unfunded
            vesting_payment.status = VestingPaymentStatus::Unfunded;

            vesting_payment
        }
    };

    VESTING_PAYMENT.save(deps.storage, &vp)?;

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
        ExecuteMsg::Distribute {} => execute_distribute(env, deps),
        ExecuteMsg::Cancel {} => execute_cancel_vesting_payment(env, deps, info),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::Delegate { validator, amount } => {
            execute_delegate(env, deps, info, validator, amount)
        }
        ExecuteMsg::Undelegate { validator, amount } => {
            execute_undelegate(env, deps, info, validator, amount)
        }
        ExecuteMsg::WithdrawDelegatorReward { validator } => {
            execute_withdraw_rewards(env, deps, validator)
        }
    }
}

pub fn execute_receive_cw20(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only accepts cw20 tokens
    nonpayable(&info)?;

    let msg: ReceiveMsg = from_binary(&receive_msg.msg)?;

    match msg {
        ReceiveMsg::Fund {} => {
            let mut vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

            // Check amount sent matches amount in vesting payment
            if vesting_payment.amount != receive_msg.amount {
                return Err(ContractError::AmountDoesNotMatch);
            }

            // Check that the Cw20 specified in the denom *must be* matches sender
            if info.sender != vesting_payment.denom.to_string() {
                return Err(ContractError::Cw20DoesNotMatch);
            }

            // Check status is Unfunded to prevent funding twice
            if vesting_payment.status != VestingPaymentStatus::Unfunded {
                return Err(ContractError::AlreadyFunded);
            }

            // Update vesting payment status
            vesting_payment.status = VestingPaymentStatus::Active;
            VESTING_PAYMENT.save(deps.storage, &vesting_payment)?;

            Ok(Response::new()
                .add_attribute("method", "fund_cw20_vesting_payment")
                .add_attribute("recipient", vesting_payment.recipient.to_string()))
        }
    }
}

pub fn execute_cancel_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Check sender is contract owner
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Get total delegations and rewards across all validators
    let total_delegations = STAKED_VESTING_BY_VALIDATOR
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| -> StdResult<Uint128> { Ok(res?.1) })
        .sum::<StdResult<Uint128>>()?;
    let validator_rewards = VALIDATORS_REWARDS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| -> StdResult<Decimal> { Ok(res?.1.rewards_per_token) })
        .sum::<StdResult<Decimal>>()?;

    let vesting_payment =
        VESTING_PAYMENT.update(deps.storage, |mut vp| -> Result<_, ContractError> {
            // Check if contract status is active
            if vp.status != VestingPaymentStatus::Active {
                return Err(ContractError::NotActive);
            }

            // Set status based on whether any funds are staked
            if vp.staked_amount == Uint128::zero() {
                vp.status = VestingPaymentStatus::Canceled;
            } else {
                vp.status = VestingPaymentStatus::CanceledAndUnbonding;
                vp.canceled_at_time = Some(env.block.time.seconds());

                // Update pending rewards
                vp.calc_pending_rewards(validator_rewards, total_delegations)?;
            }
            Ok(vp)
        })?;

    // Handle edge case if funds are staked at the time of cancelation
    if vesting_payment.status == VestingPaymentStatus::CanceledAndUnbonding {
        let denom = deps.querier.query_bonded_denom()?;

        // Unbond any staked funds
        let mut undelegate_msgs = STAKED_VESTING_BY_VALIDATOR
            .range(deps.storage, None, None, Order::Ascending)
            .flat_map(|svc| -> StdResult<CosmosMsg> {
                let (validator, amount) = svc?;
                Ok(CosmosMsg::Staking(StakingMsg::Undelegate {
                    validator,
                    amount: Coin {
                        denom: denom.clone(),
                        amount,
                    },
                }))
            })
            .collect::<Vec<CosmosMsg>>();

        msgs.append(&mut undelegate_msgs);
    } else {
        // In no funds are staked, we distibute contract funds
        // Check if funds have vested or there are staking rewards
        let vested_amount =
            vesting_payment.get_vested_amount_by_seconds(env.block.time.seconds())?;
        let staking_rewards = vesting_payment.get_pending_rewards()?;

        // Transfer any remaining unvested amount to the owner
        let unvested = vesting_payment.amount.checked_sub(vested_amount)?;
        if unvested != Uint128::zero() {
            let transfer_unvested_to_owner_msg = vesting_payment
                .denom
                .get_transfer_to_message(&info.sender, unvested)?;
            msgs.push(transfer_unvested_to_owner_msg);
        }

        // Transfer any vested amount to the original recipient
        if vested_amount != Uint128::zero() {
            let transfer_vested_to_recipient_msg = vesting_payment
                .denom
                .get_transfer_to_message(&vesting_payment.recipient, vested_amount)?;
            msgs.push(transfer_vested_to_recipient_msg);
        }

        // Transfer any pending rewards to original recipient
        if staking_rewards != Uint128::zero() {
            let transfer_rewards_to_recipient_msg = vesting_payment
                .denom
                .get_transfer_to_message(&vesting_payment.recipient, staking_rewards)?;
            msgs.push(transfer_rewards_to_recipient_msg);
        }
    }

    Ok(Response::new()
        .add_attribute("method", "remove_vesting_payment")
        .add_attribute("owner", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_messages(msgs))
}

pub fn execute_distribute(env: Env, deps: DepsMut) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

    // Get vested amount based on stats
    let vested_amount = match vesting_payment.status {
        // If canceled and unbonding use canceled time
        VestingPaymentStatus::CanceledAndUnbonding => vesting_payment
            .get_vested_amount_by_seconds(
                vesting_payment
                    .canceled_at_time
                    .unwrap_or_else(|| env.block.time.seconds()),
            )?,
        // Otherwise use current block time
        _ => vesting_payment.get_vested_amount_by_seconds(env.block.time.seconds())?,
    };
    let staking_rewards = vesting_payment.get_pending_rewards()?;

    // Check there are funds to distribute
    if vested_amount == Uint128::zero() && staking_rewards == Uint128::zero() {
        return Err(ContractError::NoFundsToClaim);
    }

    // Update Vesting Payment with claimed amount
    VESTING_PAYMENT.update(deps.storage, |mut vp| -> Result<_, ContractError> {
        // Update amounts
        vp.amount -= vested_amount;
        vp.claimed_amount += vested_amount;
        vp.rewards.pending = Decimal::zero();

        // Check if canceled and unbonding
        if vp.status == VestingPaymentStatus::CanceledAndUnbonding {
            // Set status to canceled as funds have unbonded if this executes
            vp.status = VestingPaymentStatus::Canceled
        }

        // If the amount remaining in contract goes to zero, update status
        if vp.amount == Uint128::zero() {
            vp.status = VestingPaymentStatus::FullyVested
        }
        Ok(vp)
    })?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Get transfer message for recipient payout
    let total_payout = vested_amount.checked_add(staking_rewards)?;
    if total_payout != Uint128::zero() {
        let transfer_msg = vesting_payment
            .denom
            .get_transfer_to_message(&vesting_payment.recipient, total_payout)?;
        msgs.push(transfer_msg);
    }

    // Get owner's payout message in event of canceled and unbonding
    if vesting_payment.status == VestingPaymentStatus::CanceledAndUnbonding {
        let unvested_amount = vesting_payment.amount.checked_sub(vested_amount)?;
        let owner = cw_ownable::get_ownership(deps.storage)?;
        if let Some(owner) = owner.owner {
            if unvested_amount != Uint128::zero() {
                let withdraw_unbonded_msg = vesting_payment
                    .denom
                    .get_transfer_to_message(&owner, unvested_amount)?;
                msgs.push(withdraw_unbonded_msg);
            }
        }
    }

    Ok(Response::new()
        .add_attribute("method", "distribute")
        .add_attribute("vested_amount", vested_amount)
        .add_messages(msgs))
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
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Non-payable as this method delegates funds in the contract
    nonpayable(&info)?;

    let vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

    // Check status is active
    if vesting_payment.status != VestingPaymentStatus::Active {
        return Err(ContractError::NotActive);
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
    VESTING_PAYMENT.update(deps.storage, |mut vp| -> Result<_, ContractError> {
        // Check stake amount is not greater than vesting payment amount
        if amount > vp.amount {
            return Err(ContractError::NotEnoughFunds {});
        }

        // Check that amount has not already been staked
        vp.amount.checked_sub(vp.staked_amount + amount)?;

        // Update amounts
        vp.staked_amount += amount;
        Ok(vp)
    })?;

    // Save a record of staking this vesting payment amount to a validator
    match STAKED_VESTING_BY_VALIDATOR.may_load(deps.storage, &validator)? {
        Some(staked_amount) => {
            STAKED_VESTING_BY_VALIDATOR.save(deps.storage, &validator, &(staked_amount + amount))
        }
        None => STAKED_VESTING_BY_VALIDATOR.save(deps.storage, &validator, &amount),
    }?;

    // If its a first delegation to a validator, we set validator rewards to 0
    if (VALIDATORS_REWARDS.may_load(deps.storage, &validator)?).is_none() {
        VALIDATORS_REWARDS.save(deps.storage, &validator, &ValidatorRewards::default())?;
    }

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
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

    // Check sender is the recipient of the vesting payment
    if info.sender != vesting_payment.recipient {
        return Err(ContractError::Unauthorized);
    }

    let delegations = STAKED_VESTING_BY_VALIDATOR
        .may_load(deps.storage, &validator)?
        .ok_or(ContractError::NoDelegationsForValidator {})?;

    let validator_rewards = VALIDATORS_REWARDS.load(deps.storage, &validator)?;

    // Update vesting payment with amount of staked rewards
    VESTING_PAYMENT.update(deps.storage, |mut vp| -> Result<_, ContractError> {
        // Check unstake amount is not greater than vesting payment amount
        if amount > vp.staked_amount {
            return Err(ContractError::NotEnoughFunds {});
        }

        // Update pending rewards
        vp.calc_pending_rewards(validator_rewards.rewards_per_token, delegations)?;

        // Update amounts
        vp.staked_amount -= amount;
        Ok(vp)
    })?;

    let denom = deps.querier.query_bonded_denom()?;

    // Create message to delegate the underlying tokens
    let msg = StakingMsg::Undelegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    // Update amount delegated to validator
    STAKED_VESTING_BY_VALIDATOR.save(deps.storage, &validator, &(delegations - amount))?;

    Ok(Response::default().add_message(msg))
}

pub fn execute_withdraw_rewards(
    env: Env,
    deps: DepsMut,
    validator: String,
) -> Result<Response, ContractError> {
    // Query full delegation to get the total rewards amount
    let delegation_query = deps
        .querier
        .query_delegation(env.contract.address, validator.clone())?;

    // Total rewards we have from this validator
    let total_accumulated_rewards = &match delegation_query {
        Some(delegation) => delegation.accumulated_rewards,
        None => return Err(ContractError::NoDelegationsForValidator {}),
    };

    // Check to make sure there are rewards
    if total_accumulated_rewards.is_empty() || total_accumulated_rewards[0].amount.is_zero() {
        return Err(ContractError::ZeroRewardsToSend {});
    }

    let total_accumulated_rewards = &total_accumulated_rewards[0];

    // Get total delegations across all validators
    let total_delegations = STAKED_VESTING_BY_VALIDATOR
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| -> StdResult<Uint128> { Ok(res?.1) })
        .sum::<StdResult<Uint128>>()?;

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
    let withdraw_msg = DistributionMsg::WithdrawDelegatorReward { validator };

    Ok(Response::default().add_message(withdraw_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => to_binary(&VESTING_PAYMENT.load(deps.storage)?),
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
