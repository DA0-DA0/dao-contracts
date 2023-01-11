#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, Order, Response, StakingMsg, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_utils::{must_pay, nonpayable};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{
    VestingPayment, VestingPaymentStatus, STAKED_VESTING_BY_VALIDATOR, VESTING_PAYMENT,
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

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Check all params and create vesting payment with status based on denom
    let vp = match msg.params.denom {
        UncheckedDenom::Native(ref denom) => {
            // Validate all vesting payment params
            let checked_params = msg.clone().params.into_checked(deps.as_ref())?;

            // Check amount sent matches amount in vesting payment
            if checked_params.amount != must_pay(&info, &checked_params.denom.to_string())? {
                return Err(ContractError::AmountDoesNotMatch);
            }

            // If the native denom matches the bonded denom, set withdraw address
            // to the recipient so that any staking rewards go to the recipient
            let local_denom = deps.querier.query_bonded_denom()?;
            if local_denom == *denom {
                msgs.push(CosmosMsg::Distribution(
                    DistributionMsg::SetWithdrawAddress {
                        address: checked_params.recipient.to_string(),
                    },
                ))
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
        .add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string()))
        .add_messages(msgs))
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
        ExecuteMsg::Redelegate {
            src_validator,
            dst_validator,
            amount,
        } => execute_redelegate(env, deps, info, src_validator, dst_validator, amount),
        ExecuteMsg::Undelegate { validator, amount } => {
            execute_undelegate(env, deps, info, validator, amount)
        }
        ExecuteMsg::SetWithdrawAddress { address } => {
            execute_set_withdraw_address(deps, info, address)
        }
        ExecuteMsg::WithdrawDelegatorReward { validator } => execute_withdraw_rewards(validator),
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
                // Set canceled at time for use when distributing unbonded funds
                vp.canceled_at_time = Some(env.block.time.seconds());
            }
            Ok(vp)
        })?;

    // In no funds are staked, we distibute contract funds
    // Check if funds have vested or there are staking rewards
    let vested_amount = vesting_payment.get_vested_amount_by_seconds(env.block.time.seconds())?;

    // Transfer any vested amount to the original recipient
    if vested_amount != Uint128::zero() {
        let transfer_vested_to_recipient_msg = vesting_payment
            .denom
            .get_transfer_to_message(&vesting_payment.recipient, vested_amount)?;
        msgs.push(transfer_vested_to_recipient_msg);
    }

    // Transfer any remaining unvested amount to the owner
    let unvested = vesting_payment.amount.checked_sub(vested_amount)?;
    if unvested != Uint128::zero() {
        let transfer_unvested_to_owner_msg = vesting_payment
            .denom
            .get_transfer_to_message(&info.sender, unvested)?;
        msgs.push(transfer_unvested_to_owner_msg);
    }

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
    }

    Ok(Response::new()
        .add_attribute("method", "remove_vesting_payment")
        .add_attribute("owner", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_messages(msgs))
}

pub fn execute_distribute(env: Env, deps: DepsMut) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

    // Get vested amount based on vesting payment status
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

    // Check there are funds to distribute
    if vested_amount == Uint128::zero() {
        return Err(ContractError::NoFundsToClaim);
    }

    // Update Vesting Payment with claimed amount and the correct status
    VESTING_PAYMENT.update(deps.storage, |mut vp| -> Result<_, ContractError> {
        // Decrease vesting amount
        vp.amount = vp.amount.checked_sub(vested_amount)?;
        // Increase the claimed amount
        vp.claimed_amount = vp.claimed_amount.checked_add(vested_amount)?;

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
    if vested_amount != Uint128::zero() {
        let transfer_msg = vesting_payment
            .denom
            .get_transfer_to_message(&vesting_payment.recipient, vested_amount)?;
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
        vp.staked_amount += vp.staked_amount.checked_add(amount)?;
        Ok(vp)
    })?;

    // Save a record of staking this vesting payment amount to a validator
    match STAKED_VESTING_BY_VALIDATOR.may_load(deps.storage, &validator)? {
        // If already staked to this validator, increase staked amount
        Some(staked_amount) => STAKED_VESTING_BY_VALIDATOR.save(
            deps.storage,
            &validator,
            &staked_amount.checked_add(amount)?,
        ),
        // If not currently staked to this validator save a new record with staked amount
        None => STAKED_VESTING_BY_VALIDATOR.save(deps.storage, &validator, &amount),
    }?;

    // Create message to delegate the underlying tokens
    let msg = StakingMsg::Delegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    Ok(Response::new()
        .add_attribute("method", "delegate")
        .add_attribute("amount", amount.to_string())
        .add_attribute("validator", validator)
        .add_message(msg))
}

pub fn execute_redelegate(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    src_validator: String,
    dst_validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Non-payable as this method redelegates funds in the contract
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

    // Load source validator, subtract redelegated amount
    let amount_staked_src = STAKED_VESTING_BY_VALIDATOR.load(deps.storage, &src_validator)?;
    STAKED_VESTING_BY_VALIDATOR.save(
        deps.storage,
        &src_validator,
        &amount_staked_src.checked_sub(amount)?,
    )?;

    // Load destination validator, increase amount
    match STAKED_VESTING_BY_VALIDATOR.may_load(deps.storage, &dst_validator)? {
        // If already staked to this validator, increase staked amount
        Some(staked_amount) => STAKED_VESTING_BY_VALIDATOR.save(
            deps.storage,
            &src_validator,
            &staked_amount.checked_add(amount)?,
        ),
        // If not currently staked to this validator save a new record with staked amount
        None => STAKED_VESTING_BY_VALIDATOR.save(deps.storage, &dst_validator, &amount),
    }?;

    // Create message to delegate the underlying tokens
    let msg = StakingMsg::Redelegate {
        src_validator,
        dst_validator,
        amount: Coin { denom, amount },
    };

    Ok(Response::new()
        .add_attribute("method", "redelegate")
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

    // Update vesting payment with amount of staked rewards
    VESTING_PAYMENT.update(deps.storage, |mut vp| -> Result<_, ContractError> {
        // Check unstake amount is not greater than vesting payment amount
        if amount > vp.staked_amount {
            return Err(ContractError::NotEnoughFunds {});
        }

        // Update amounts
        vp.staked_amount = vp.staked_amount.checked_sub(amount)?;
        Ok(vp)
    })?;

    let denom = deps.querier.query_bonded_denom()?;

    // Create message to delegate the underlying tokens
    let msg = StakingMsg::Undelegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    // Update amount delegated to validator
    STAKED_VESTING_BY_VALIDATOR.update(deps.storage, &validator, |staked_amount| {
        match staked_amount {
            Some(staked_amount) => Ok(staked_amount.checked_sub(amount)?),
            None => Err(ContractError::NoDelegationsForValidator {}),
        }
    })?;

    Ok(Response::default().add_message(msg))
}

pub fn execute_set_withdraw_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

    // Check sender is the recipient of the vesting payment
    if info.sender != vesting_payment.recipient {
        return Err(ContractError::Unauthorized);
    }

    // Set withdraw address
    let msg = DistributionMsg::SetWithdrawAddress { address };

    Ok(Response::default().add_message(msg))
}

pub fn execute_withdraw_rewards(validator: String) -> Result<Response, ContractError> {
    // Withdraw rewards from validator
    let withdraw_msg = DistributionMsg::WithdrawDelegatorReward { validator };

    Ok(Response::default().add_message(withdraw_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => to_binary(&VESTING_PAYMENT.load(deps.storage)?),
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::VestedAmount {} => {
            let vesting_payment = VESTING_PAYMENT.load(deps.storage)?;

            to_binary(
                &vesting_payment
                    .vesting_schedule
                    .value(env.block.time.seconds()),
            )
        }
    }
}
