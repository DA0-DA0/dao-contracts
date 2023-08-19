use std::cmp::Ordering;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_paginate_storage::paginate_map_values;
use cw_utils::{may_pay, must_pay};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{Bounty, BountyStatus, BOUNTIES, ID},
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-bounties";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;

    // Set the contract owner
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

    // Initialize the next ID
    ID.save(deps.storage, &0)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Only the owner can execute messages on this contract
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    match msg {
        ExecuteMsg::Close { id } => close(deps, env, info, id),
        ExecuteMsg::Create {
            amount,
            title,
            description,
        } => create(deps, env, info, amount, title, description),
        ExecuteMsg::PayOut { id, recipient } => pay_out(deps, env, id, recipient),
        ExecuteMsg::Update {
            id,
            amount,
            title,
            description,
        } => update(deps, env, info, id, amount, title, description),
        ExecuteMsg::UpdateOwnership(action) => update_owner(deps, info, env, action),
    }
}

pub fn create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Coin,
    title: String,
    description: Option<String>,
) -> Result<Response, ContractError> {
    // Check funds sent match the bounty amount specified
    let sent_amount = must_pay(&info, &amount.denom)?;
    if sent_amount != amount.amount {
        return Err(ContractError::InvalidAmount {
            expected: amount.amount,
            actual: sent_amount,
        });
    };

    // Check bounty title is not empty string
    if title.is_empty() {
        return Err(ContractError::EmptyTitle {});
    }

    // Increment and get the next bounty ID
    let id = ID.update(deps.storage, |mut id| -> StdResult<u64> {
        id += 1;
        Ok(id)
    })?;

    // Save the bounty
    BOUNTIES.save(
        deps.storage,
        id,
        &Bounty {
            id,
            amount,
            title,
            description,
            status: BountyStatus::Open,
            created_at: env.block.time.seconds(),
            updated_at: None,
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "create_bounty")
        .add_attribute("id", id.to_string()))
}

pub fn close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    // Check bounty exists
    let mut bounty = BOUNTIES.load(deps.storage, id)?;

    // Check bounty is open
    if bounty.status != BountyStatus::Open {
        return Err(ContractError::NotOpen {});
    };

    bounty.status = BountyStatus::Closed {
        closed_at: env.block.time.seconds(),
    };
    BOUNTIES.save(deps.storage, id, &bounty)?;

    // Pay out remaining funds to owner
    // Only owner can call this, so sender is owner
    let msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![bounty.amount],
    };

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("action", "close_bounty"))
}

pub fn pay_out(
    deps: DepsMut,
    env: Env,
    id: u64,
    recipient: String,
) -> Result<Response, ContractError> {
    // Check bounty exists
    let mut bounty = BOUNTIES.load(deps.storage, id)?;

    // Check bounty is open
    if bounty.status != BountyStatus::Open {
        return Err(ContractError::NotOpen {});
    }

    // Validate recipient address
    deps.api.addr_validate(&recipient)?;

    // Set bounty status to claimed
    bounty.status = BountyStatus::Claimed {
        claimed_by: recipient.clone(),
        claimed_at: env.block.time.seconds(),
    };
    BOUNTIES.save(deps.storage, id, &bounty)?;

    // Message to pay out remaining funds to recipient
    let msg = BankMsg::Send {
        to_address: recipient.clone(),
        amount: vec![bounty.clone().amount],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "pay_out_bounty")
        .add_attribute("bounty_id", id.to_string())
        .add_attribute("amount", bounty.amount.to_string())
        .add_attribute("recipient", recipient))
}

pub fn update(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
    new_amount: Coin,
    title: String,
    description: Option<String>,
) -> Result<Response, ContractError> {
    // Check bounty exists
    let bounty = BOUNTIES.load(deps.storage, id)?;

    // Check bounty is open
    if bounty.status != BountyStatus::Open {
        return Err(ContractError::NotOpen {});
    }

    // Update bounty
    BOUNTIES.save(
        deps.storage,
        bounty.id,
        &Bounty {
            id: bounty.id,
            amount: new_amount.clone(),
            title,
            description,
            status: bounty.status,
            created_at: bounty.created_at,
            updated_at: Some(env.block.time.seconds()),
        },
    )?;

    let res = Response::new()
        .add_attribute("action", "update_bounty")
        .add_attribute("bounty_id", id.to_string())
        .add_attribute("amount", new_amount.amount.to_string());

    // check if new amount has different denom
    if new_amount.denom != bounty.amount.denom {
        // If denom is different, check funds sent match new amount
        let sent_amount = must_pay(&info, &new_amount.denom)?;
        if sent_amount != new_amount.amount {
            return Err(ContractError::InvalidAmount {
                expected: new_amount.amount,
                actual: sent_amount,
            });
        }
        // send back old amount
        let msg = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: bounty.amount.denom,
                amount: bounty.amount.amount,
            }],
        };

        return Ok(res.add_message(msg));
    };

    // Check if amount is greater or less than original amount
    let old_amount = bounty.amount.clone();
    match new_amount.amount.cmp(&old_amount.amount) {
        Ordering::Greater => {
            // If new amount is greater, check funds sent plus
            // original amount match new amount
            let sent_amount = must_pay(&info, &new_amount.denom)?;
            if sent_amount + old_amount.amount != new_amount.amount {
                return Err(ContractError::InvalidAmount {
                    expected: new_amount.amount - old_amount.amount,
                    actual: sent_amount + old_amount.amount,
                });
            }
            Ok(res)
        }
        Ordering::Less => {
            // If new amount is less, pay out difference to owner
            // in case owner accidentally sent funds, send back as well
            let funds_send = may_pay(&info, &bounty.amount.denom)?;
            let diff = old_amount.amount - new_amount.amount + funds_send;
            let msg = BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: old_amount.denom,
                    amount: diff,
                }],
            };

            Ok(res.add_message(msg))
        }
        Ordering::Equal => {
            // If the new amount hasn't changed we return the response
            Ok(res)
        }
    }
}

pub fn update_owner(
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
        QueryMsg::Bounty { id } => to_binary(&BOUNTIES.load(deps.storage, id)?),
        QueryMsg::Bounties { start_after, limit } => to_binary(&paginate_map_values(
            deps,
            &BOUNTIES,
            start_after,
            limit,
            Order::Descending,
        )?),
        QueryMsg::Count {} => to_binary(&ID.load(deps.storage)?),
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
