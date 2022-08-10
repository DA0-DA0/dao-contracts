#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetPaymentsResponse, InstantiateMsg, QueryMsg};
use crate::state::{CheckedPayment, PaymentState, ADMIN, PAYMENTS};
use cw20::Cw20ExecuteMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin = deps.api.addr_validate(&msg.admin_address)?;
    ADMIN.save(deps.storage, &admin)?;
    msg.payments
        .into_iter()
        .try_for_each::<_, Result<(), ContractError>>(|p| {
            let checked_payment = p.into_checked(deps.as_ref())?;
            PAYMENTS.save(deps.storage, &checked_payment.recipient, &checked_payment)?;
            Ok(())
        })?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::PausePayments { recipient } => pause_payment(deps, env, info, recipient),
        ExecuteMsg::ResumePayments { recipient } => resume_payment(deps, env, info, recipient),
        ExecuteMsg::SchedulePayments {} => todo!(),
    }
}

pub fn pause_payment(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }

    let recipient_address = deps.api.addr_validate(&recipient)?;
    PAYMENTS.update::<_, ContractError>(deps.storage, &recipient_address, |payment| {
        let mut payment = payment.ok_or_else(|| ContractError::PayeeNotFound {
            addr: recipient_address.clone(),
        })?;
        payment.state = PaymentState::Paused;
        Ok(payment)
    })?;

    Ok(Response::new().add_attribute("payment_paused_for", recipient))
}

pub fn resume_payment(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }

    let recipient_address = deps.api.addr_validate(&recipient)?;
    PAYMENTS.update::<_, ContractError>(deps.storage, &recipient_address, |payment| {
        let mut payment = payment.ok_or_else(|| ContractError::PayeeNotFound {
            addr: recipient_address.clone(),
        })?;
        payment.state = PaymentState::Active;
        Ok(payment)
    })?;

    Ok(Response::new().add_attribute("payment_resumed_for", recipient))
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: support multiple active payments per user and recurring payments
    let mut payment: CheckedPayment =
        PAYMENTS
            .may_load(deps.storage, &info.sender)?
            .ok_or_else(|| ContractError::PayeeNotFound {
                addr: info.sender.clone(),
            })?;

    // Payment must be active and vested in order to be claimed.
    if !(payment.state == PaymentState::Active && payment.vesting_time.is_expired(&env.block)) {
        return Err(ContractError::NothingToClaim {});
    }

    // Update payment state
    payment.state = PaymentState::Claimed;
    PAYMENTS.save(deps.storage, &info.sender, &payment)?;

    // Get cosmos payment message
    let payment_msg: CosmosMsg = get_payment_message(&payment)?;

    Ok(Response::new().add_message(payment_msg))
}

pub fn get_payment_message(p: &CheckedPayment) -> Result<CosmosMsg, ContractError> {
    match p.token_address {
        Some(_) => get_token_payment(p),
        None => get_native_payment(p),
    }
}

pub fn get_token_payment(p: &CheckedPayment) -> Result<CosmosMsg, ContractError> {
    let transfer_cw20_msg = Cw20ExecuteMsg::Transfer {
        recipient: p.recipient.to_string(),
        amount: p.amount,
    };

    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: p.token_address.clone().unwrap().to_string(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };

    Ok(exec_cw20_transfer.into())
}

pub fn get_native_payment(p: &CheckedPayment) -> Result<CosmosMsg, ContractError> {
    let denom = p.denom.as_ref().ok_or(ContractError::NotNativePayment {})?;
    let transfer_bank_msg = cosmwasm_std::BankMsg::Send {
        to_address: p.recipient.clone().into_string(),
        amount: vec![Coin {
            denom: denom.to_string(),
            amount: p.amount,
        }],
    };

    Ok(transfer_bank_msg.into())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPayments {} => to_binary(&query_payments(deps)?),
    }
}

fn query_payments(deps: Deps) -> StdResult<GetPaymentsResponse> {
    // TODO: paginate
    let payments = PAYMENTS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| Ok(res?.1))
        .collect::<StdResult<Vec<CheckedPayment>>>()?;

    Ok(GetPaymentsResponse { payments })
}
