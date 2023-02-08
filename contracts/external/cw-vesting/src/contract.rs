#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, Response, StakingMsg, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;
use cw_utils::{must_pay, nonpayable};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::PAYMENT;
use crate::vesting::{Status, VestInit};

const CONTRACT_NAME: &str = "crates.io:cw-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    let denom = msg.denom.into_checked(deps.as_ref())?;
    let recipient = deps.api.addr_validate(&msg.recipient)?;
    let start_time = msg.start_time.unwrap_or(env.block.time);
    let vest = PAYMENT.initialize(
        deps.storage,
        VestInit {
            total: msg.total,
            schedule: msg.schedule,
            start_time,
            duration_seconds: msg.duration_seconds,
            denom,
            recipient,
            title: msg.title,
            description: msg.description,
        },
    )?;

    let resp = match vest.denom {
        CheckedDenom::Native(ref denom) => {
            let sent = must_pay(&info, denom)?;
            if vest.total() != sent {
                return Err(ContractError::WrongFundAmount {
                    sent,
                    expected: vest.total(),
                });
            }
            PAYMENT.set_funded(deps.storage)?;

            // If the payment denomination is the same as the native
            // denomination, set the staking rewards receiver to the
            // payment receiver so that when they stake vested tokens
            // they receive the rewards.
            if denom.as_str() == deps.querier.query_bonded_denom()? {
                Some(CosmosMsg::Distribution(
                    DistributionMsg::SetWithdrawAddress {
                        address: vest.recipient.to_string(),
                    },
                ))
            } else {
                None
            }
        }
        CheckedDenom::Cw20(_) => {
            nonpayable(&info)?; // Funding happens in ExecuteMsg::Receive.
            None
        }
    };

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string()))
        .add_messages(resp))
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
        ExecuteMsg::Distribute { amount } => execute_distribute(env, deps, amount),
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
        ExecuteMsg::WithdrawCanceledPayment { amount } => {
            execute_withdraw_canceled(deps, env, info, amount)
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
            let vest = PAYMENT.get_vest(deps.storage)?;

            if vest.total() != receive_msg.amount {
                return Err(ContractError::WrongFundAmount {
                    sent: receive_msg.amount,
                    expected: vest.total(),
                });
            } // correct amount

            if !vest.denom.is_cw20(&info.sender) {
                return Err(ContractError::WrongCw20);
            } // correct denom

            if vest.status != Status::Unfunded {
                return Err(ContractError::Funded {});
            } // correct status

            PAYMENT.set_funded(deps.storage)?;

            Ok(Response::new()
                .add_attribute("method", "fund_cw20_vesting_payment")
                .add_attribute("receiver", vest.recipient.to_string()))
        }
    }
}

pub fn execute_cancel_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let msgs = PAYMENT.cancel(deps.storage, &env, &info.sender)?;

    Ok(Response::new()
        .add_attribute("method", "remove_vesting_payment")
        .add_attribute("owner", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_messages(msgs))
}

pub fn execute_distribute(
    env: Env,
    deps: DepsMut,
    request: Option<Uint128>,
) -> Result<Response, ContractError> {
    let msg = PAYMENT.distribute(deps.storage, &env.block.time, request)?;

    Ok(Response::new()
        .add_attribute("method", "distribute")
        .add_message(msg))
}

pub fn execute_withdraw_canceled(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let msg = PAYMENT.withdraw_canceled(deps.storage, &env.block.time, amount, &info.sender)?;

    Ok(Response::new()
        .add_attribute("method", "withdraw_canceled")
        .add_message(msg))
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
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let vest = PAYMENT.get_vest(deps.storage)?;

    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded {}),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        Status::Canceled { .. } => return Err(ContractError::Cancelled),
    }

    let denom = match vest.denom {
        CheckedDenom::Cw20(_) => {
            return Err(ContractError::NotStakeable);
        }
        CheckedDenom::Native(denom) => {
            let local_denom = deps.querier.query_bonded_denom()?;
            if local_denom != denom {
                return Err(ContractError::NotStakeable);
            } else {
                denom
            }
        }
    };

    PAYMENT.delegate(deps.storage, &env.block.time, amount)?;

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
    nonpayable(&info)?;

    let vest = PAYMENT.get_vest(deps.storage)?;

    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded {}),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        Status::Canceled { .. } => cw_ownable::assert_owner(deps.storage, &info.sender)?,
    }

    let denom = match vest.denom {
        CheckedDenom::Cw20(_) => {
            return Err(ContractError::NotStakeable);
        }
        CheckedDenom::Native(denom) => {
            let local_denom = deps.querier.query_bonded_denom()?;
            if local_denom != denom {
                return Err(ContractError::NotStakeable);
            } else {
                denom
            }
        }
    };

    let msg = StakingMsg::Redelegate {
        src_validator: src_validator.clone(),
        dst_validator: dst_validator.clone(),
        amount: Coin { denom, amount },
    };

    Ok(Response::new()
        .add_attribute("method", "redelegate")
        .add_attribute("amount", amount.to_string())
        .add_attribute("src_validator", src_validator)
        .add_attribute("dst_validator", dst_validator)
        .add_message(msg))
}

pub fn execute_undelegate(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let vest = PAYMENT.get_vest(deps.storage)?;

    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded {}),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        // while canceled either the owner or the receiver may
        // undelegate so long as they have funds to be settled.
        Status::Canceled { owner_withdrawable } => {
            if vest.vested(&env.block.time) != vest.claimed && info.sender == vest.recipient {
            } else if !owner_withdrawable.is_zero() {
                cw_ownable::assert_owner(deps.storage, &info.sender)?;
            } else {
                return Err(ContractError::Cancelled {});
            }
        }
    };

    PAYMENT.undelegate(deps.storage, deps.querier, &env.block.time, amount)?;

    let denom = deps.querier.query_bonded_denom()?;

    let msg = StakingMsg::Undelegate {
        validator,
        amount: Coin { denom, amount },
    };

    Ok(Response::default().add_message(msg))
}

pub fn execute_set_withdraw_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let vest = PAYMENT.get_vest(deps.storage)?;
    match vest.status {
        Status::Unfunded | Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        // In the cancelled state the owner is receiving staking
        // rewards and may update the withdraw address.
        Status::Canceled { .. } => cw_ownable::assert_owner(deps.storage, &info.sender)?,
    }

    let msg = DistributionMsg::SetWithdrawAddress {
        address: address.clone(),
    };

    Ok(Response::default()
        .add_attribute("method", "set_withdraw_address")
        .add_attribute("address", address)
        .add_message(msg))
}

pub fn execute_withdraw_rewards(validator: String) -> Result<Response, ContractError> {
    let withdraw_msg = DistributionMsg::WithdrawDelegatorReward { validator };

    Ok(Response::default().add_message(withdraw_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Vest {} => to_binary(&PAYMENT.get_vest(deps.storage)?),
    }
}
