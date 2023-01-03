#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_paginate::paginate_map_values;
use cw_utils::must_pay;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{UncheckedVestingParams, VestingPayment, VESTING_PAYMENTS, VESTING_PAYMENT_SEQ};

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
        ExecuteMsg::Delegate {} => unimplemented!(),
        ExecuteMsg::Redelgate {} => unimplemented!(),
        ExecuteMsg::Undelegate {} => unimplemented!(),
        ExecuteMsg::WithdrawRewards {} => unimplemented!(),
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

    let vesting_payment = VESTING_PAYMENTS.may_load(deps.storage, id)?.ok_or(
        ContractError::VestingPaymentNotFound {
            vesting_payment_id: id,
        },
    )?;

    VESTING_PAYMENTS.remove(deps.storage, id);

    // Transfer any remaining amount to the owner
    let transfer_to_contract_owner_msg = vesting_payment
        .denom
        .get_transfer_to_message(&info.sender, vesting_payment.amount)?;

    Ok(Response::new()
        .add_attribute("method", "remove_vesting_payment")
        .add_attribute("vesting_payment_id", id.to_string())
        .add_attribute("owner", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_message(transfer_to_contract_owner_msg))
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
        // TODO better error message
        return Err(ContractError::Unauthorized {});
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
                // TODO better error message
                return Err(ContractError::Unauthorized {});
            }

            // Check that the Cw20 specified in the denom *must be* matches sender
            if info.sender.to_string() != checked_params.denom.to_string() {
                // TODO better error message
                return Err(ContractError::Unauthorized {});
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

pub fn execute_distribute(env: Env, deps: DepsMut, id: u64) -> Result<Response, ContractError> {
    let vesting_payment = VESTING_PAYMENTS.may_load(deps.storage, id)?.ok_or(
        ContractError::VestingPaymentNotFound {
            vesting_payment_id: id,
        },
    )?;

    let vesting_funds = vesting_payment
        .vesting_schedule
        .value(env.block.time.seconds());
    let vested_amount = vesting_payment.amount - vesting_funds;

    if vested_amount == Uint128::zero() {
        return Err(ContractError::NoFundsToClaim {
            claimed: vesting_payment.claimed_amount,
        });
    }

    // this occurs when there is a curve defined, but it is now at 0 (eg. fully vested)
    // in this case, we can safely delete it (as it will remain 0 forever)
    if vesting_funds == Uint128::zero() {
        // Contract is fully vested.
        // TODO maybe update vesting payment status?
        return Err(ContractError::FullyVested {});
    }

    // Update Vesting Payment with claimed amount
    VESTING_PAYMENTS.update(deps.storage, id, |v| -> Result<_, ContractError> {
        match v {
            Some(mut v) => {
                // TODO if this becomes zero, update status to fully vested
                v.amount -= vested_amount;
                v.claimed_amount += vested_amount;
                Ok(v)
            }
            None => Err(ContractError::VestingPaymentNotFound {
                vesting_payment_id: id,
            }),
        }
    })?;

    // Get transfer message
    let transfer_msg = vesting_payment
        .denom
        .get_transfer_to_message(&vesting_payment.recipient, vested_amount)?;

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
