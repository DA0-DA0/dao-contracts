#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::UncheckedDenom;
use cw_storage_plus::Bound;
use serde::de::Error;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, ListVestingPaymentsResponse, QueryMsg, ReceiveMsg,
    VestingParams, VestingPaymentResponse,
};
use crate::state::{Config, VestingPayment, CONFIG, VESTING_PAYMENTS, VESTING_PAYMENT_SEQ};

const CONTRACT_NAME: &str = "crates.io:cw-payroll";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = match msg.admin {
        Some(ad) => deps.api.addr_validate(&ad)?,
        None => info.sender,
    };

    let config = Config {
        admin: admin.clone(),
    };
    CONFIG.save(deps.storage, &config)?;

    VESTING_PAYMENT_SEQ.save(deps.storage, &0u64)?;

    // TODO optionally fund on instantiate?
    // match msg.create_new_vesting_schedule_params {
    //     Some(vesting_params) => execute_create_vesting_payment(env, deps, vesting_params),
    //     None => Ok(()),
    // }

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(env, deps, info, msg),
        // TODO should be able to create and fund with a native token
        ExecuteMsg::Create(vesting_params) => {
            execute_create_vesting_payment(env, deps, vesting_params)
        }
        ExecuteMsg::Distribute { id } => execute_distribute(env, deps, id),
        ExecuteMsg::Pause { id } => execute_pause_vesting_payment(env, deps, info, id),
        ExecuteMsg::Resume { id } => execute_resume_vesting_payment(env, deps, info, id),
        ExecuteMsg::Cancel { id } => execute_remove_vesting_payment(env, deps, info, id),
        ExecuteMsg::Delegate {} => unimplemented!(),
        ExecuteMsg::Redelgate {} => unimplemented!(),
        ExecuteMsg::Undelegate {} => unimplemented!(),
        ExecuteMsg::WithdrawRewards {} => unimplemented!(),
    }
}

pub fn execute_pause_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    unimplemented!()
    // // TODO check admin
    // let mut vesting_payment = VESTING_PAYMENTS
    //     .may_load(deps.storage, id)?
    //     .ok_or(ContractError::VestingPaymentNotFound { vesting_payment_id: id })?;
    // if vesting_payment.admin != info.sender {
    //     return Err(ContractError::Unauthorized {});
    // }
    // if vesting_payment.paused {
    //     return Err(ContractError::AlreadyPaused {});
    // }
    // vesting_payment.paused_time = Some(env.block.time.seconds());
    // vesting_payment.paused = true;
    // VESTING_PAYMENTS.save(deps.storage, id, &vesting_payment)?;

    // Ok(Response::new()
    //     .add_attribute("method", "pause_vesting_payment")
    //     .add_attribute("paused", vesting_payment.paused.to_string())
    //     .add_attribute("vesting_payment_id", id.to_string())
    //     .add_attribute("admin", info.sender)
    //     .add_attribute("paused_time", vesting_payment.paused_time.unwrap().to_string()))
}

pub fn execute_remove_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    unimplemented!()
    // // TODO Check that sender is admin
    // let vesting_payment = VESTING_PAYMENTS
    //     .may_load(deps.storage, id)?
    //     .ok_or(ContractError::VestingPaymentNotFound { vesting_payment_id: id })?;
    // if vesting_payment.admin != info.sender {
    //     return Err(ContractError::Unauthorized {});
    // }

    // VESTING_PAYMENTS.remove(deps.storage, id);

    // // Transfer any remaining amount to the owner
    // let transfer_to_admin_msg = vesting_payment
    //     .denom
    //     .get_transfer_to_message(&vesting_payment.admin, vesting_payment.amount)?;

    // Ok(Response::new()
    //     .add_attribute("method", "remove_vesting_payment")
    //     .add_attribute("vesting_payment_id", id.to_string())
    //     .add_attribute("admin", info.sender)
    //     .add_attribute("removed_time", env.block.time.to_string())
    //     .add_message(transfer_to_admin_msg))
}

pub fn execute_resume_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    unimplemented!()
    // // TODO check admin
    // let mut vesting_payment = VESTING_PAYMENTS
    //     .may_load(deps.storage, id)?
    //     .ok_or(ContractError::VestingPaymentNotFound { vesting_payment_id: id })?;
    // if vesting_payment.admin != info.sender {
    //     return Err(ContractError::Unauthorized {});
    // }
    // if !vesting_payment.paused {
    //     return Err(ContractError::NotPaused {});
    // }
    // vesting_payment.paused_duration = vesting_payment.calc_pause_duration(env.block.time);
    // vesting_payment.paused = false;
    // vesting_payment.paused_time = None;
    // VESTING_PAYMENTS.save(deps.storage, id, &vesting_payment)?;

    // let (_, rate_per_second) = vesting_payment.calc_distribution_rate(env.block.time)?;
    // let response = Response::new()
    //     .add_attribute("method", "resume_vesting_payment")
    //     .add_attribute("vesting_payment_id", id.to_string())
    //     .add_attribute("admin", info.sender)
    //     .add_attribute("rate_per_second", rate_per_second)
    //     .add_attribute("resume_time", env.block.time.to_string())
    //     .add_attribute(
    //         "paused_duration",
    //         vesting_payment.paused_duration.unwrap().to_string(),
    //     )
    //     .add_attribute("resume_time", env.block.time.to_string());

    // Ok(response)
}

pub fn execute_create_vesting_payment(
    _env: Env,
    deps: DepsMut,
    vesting_params: VestingParams,
) -> Result<Response, ContractError> {
    let VestingParams {
        recipient,
        amount,
        denom,
        vesting_schedule,
        title,
        description,
    } = vesting_params;

    let recipient = deps.api.addr_validate(&recipient)?;

    // if start_time > end_time {
    //     return Err(ContractError::InvalidStartTime {});
    // }
    // let block_time = env.block.time.seconds();
    // if end_time <= block_time {
    //     return Err(ContractError::InvalidEndTime {});
    // }

    let vesting_payment = VestingPayment {
        recipient: recipient.clone(),
        amount,
        claimed_amount: Uint128::zero(),
        denom,
        vesting_schedule,
        paused: false,
        title,
        description,
    };

    // Check vesting schedule
    vesting_payment.assert_schedule_vests_amount(amount)?;

    let id = VESTING_PAYMENT_SEQ.load(deps.storage)?;
    let id = id + 1;
    VESTING_PAYMENT_SEQ.save(deps.storage, &id)?;
    VESTING_PAYMENTS.save(deps.storage, id, &vesting_payment)?;

    Ok(Response::new()
        .add_attribute("method", "create_vesting_payment")
        .add_attribute("vesting_payment_id", id.to_string())
        .add_attribute("recipient", recipient))
}

pub fn execute_receive(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&info.sender.clone().into_string())?;
    let msg: ReceiveMsg = from_binary(&receive_msg.msg)?;
    // TODO check cw20 denom matches info.sender

    // TODO actually check denom in params
    let checked_denom =
        UncheckedDenom::Cw20(info.sender.to_string()).into_checked(deps.as_ref())?;

    match msg {
        ReceiveMsg::Create(msg) => execute_create_vesting_payment(env, deps, msg),
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetVestingPayment { id } => to_binary(&query_vesting_payment(deps, id)?),
        QueryMsg::ListVestingPayments { start, limit } => {
            to_binary(&query_list_vesting_payments(deps, start, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.into(),
    })
}

fn query_vesting_payment(deps: Deps, id: u64) -> StdResult<VestingPaymentResponse> {
    let vesting_payment = VESTING_PAYMENTS.load(deps.storage, id)?;
    Ok(VestingPaymentResponse {
        id,
        recipient: vesting_payment.recipient.into(),
        amount: vesting_payment.amount,
        claimed_amount: vesting_payment.claimed_amount,
        denom: vesting_payment.denom,
        vesting_schedule: vesting_payment.vesting_schedule,
        // start_time: vesting_payment.start_time,
        // end_time: vesting_payment.end_time,
        title: vesting_payment.title,
        description: vesting_payment.description,
        // paused_time: vesting_payment.paused_time,
        // paused_duration: vesting_payment.paused_duration,
        paused: vesting_payment.paused,
    })
}

fn query_list_vesting_payments(
    deps: Deps,
    start: Option<u8>,
    limit: Option<u8>,
) -> StdResult<ListVestingPaymentsResponse> {
    let start = start.map(Bound::inclusive);
    let limit = limit.unwrap_or(5);

    let vesting_payments = VESTING_PAYMENTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit.into())
        .map(map_vesting_payment)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ListVestingPaymentsResponse { vesting_payments })
}

fn map_vesting_payment(
    item: StdResult<(u64, VestingPayment)>,
) -> StdResult<VestingPaymentResponse> {
    item.map(|(id, vesting_payment)| VestingPaymentResponse {
        id,
        recipient: vesting_payment.recipient.to_string(),
        amount: vesting_payment.amount,
        claimed_amount: vesting_payment.claimed_amount,
        denom: vesting_payment.denom,
        vesting_schedule: vesting_payment.vesting_schedule,
        // start_time: vesting_payment.start_time,
        // end_time: vesting_payment.end_time,
        title: vesting_payment.title,
        description: vesting_payment.description,
        // paused_time: vesting_payment.paused_time,
        // paused_duration: vesting_payment.paused_duration,
        paused: vesting_payment.paused,
    })
}
