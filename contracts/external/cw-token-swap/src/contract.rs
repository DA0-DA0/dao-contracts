#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Item;
use cw_utils::must_pay;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StatusResponse},
    state::{CheckedCounterparty, CheckedTokenInfo, COUNTERPARTY_ONE, COUNTERPARTY_TWO},
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-token-swap";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let counterparty_one = msg.counterparty_one.into_checked(deps.as_ref())?;
    let counterparty_two = msg.counterparty_two.into_checked(deps.as_ref())?;

    if counterparty_one.address == counterparty_two.address {
        return Err(ContractError::NonDistinctCounterparties {});
    }

    COUNTERPARTY_ONE.save(deps.storage, &counterparty_one)?;
    COUNTERPARTY_TWO.save(deps.storage, &counterparty_two)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("counterparty_one", counterparty_one.address)
        .add_attribute("counterparty_two", counterparty_two.address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, info.sender, msg),
        ExecuteMsg::Fund {} => execute_fund(deps, info),
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, info),
    }
}

struct CounterpartyResponse<'a> {
    pub counterparty: CheckedCounterparty,
    pub other_counterparty: CheckedCounterparty,
    pub storage: Item<'a, CheckedCounterparty>,
}

fn get_counterparty<'a>(
    deps: Deps,
    sender: &Addr,
) -> Result<CounterpartyResponse<'a>, ContractError> {
    let counterparty_one = COUNTERPARTY_ONE.load(deps.storage)?;
    let counterparty_two = COUNTERPARTY_TWO.load(deps.storage)?;

    let (counterparty, other_counterparty, storage) = if *sender == counterparty_one.address {
        (counterparty_one, counterparty_two, COUNTERPARTY_ONE)
    } else if *sender == counterparty_two.address {
        (counterparty_two, counterparty_one, COUNTERPARTY_TWO)
    } else {
        // Contract may only be funded by a counterparty.
        return Err(ContractError::Unauthorized {});
    };

    Ok(CounterpartyResponse {
        counterparty,
        other_counterparty,
        storage,
    })
}

/// Accepts funding from COUNTERPARTY for the escrow. Distributes
/// escrow funds if both counterparties have funded the contract.
///
/// NOTE: The caller must verify that the denom of PAID is correct.
fn do_fund(
    deps: DepsMut,
    counterparty: CheckedCounterparty,
    paid: Uint128,
    expected: Uint128,
    other_counterparty: CheckedCounterparty,
    storage: Item<CheckedCounterparty>,
) -> Result<Response, ContractError> {
    if counterparty.provided {
        return Err(ContractError::AlreadyProvided {});
    }

    if paid != expected {
        return Err(ContractError::InvalidAmount {
            expected,
            actual: paid,
        });
    }

    let mut counterparty = counterparty;
    counterparty.provided = true;
    storage.save(deps.storage, &counterparty)?;

    let messages = if counterparty.provided && other_counterparty.provided {
        vec![
            counterparty
                .promise
                .into_send_message(&other_counterparty.address)?,
            other_counterparty
                .promise
                .into_send_message(&counterparty.address)?,
        ]
    } else {
        vec![]
    };

    Ok(Response::new()
        .add_attribute("method", "fund_escrow")
        .add_attribute("counterparty", counterparty.address)
        .add_messages(messages))
}

pub fn execute_receive(
    deps: DepsMut,
    token_contract: Addr,
    msg: cw20::Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&msg.sender)?;

    let CounterpartyResponse {
        counterparty,
        other_counterparty,
        storage,
    } = get_counterparty(deps.as_ref(), &sender)?;

    let (expected_payment, paid) = if let CheckedTokenInfo::Cw20 {
        contract_addr,
        amount,
    } = &counterparty.promise
    {
        if *contract_addr != token_contract {
            // Must fund with the promised tokens.
            return Err(ContractError::InvalidFunds {});
        }

        (*amount, msg.amount)
    } else {
        return Err(ContractError::InvalidFunds {});
    };

    do_fund(
        deps,
        counterparty,
        paid,
        expected_payment,
        other_counterparty,
        storage,
    )
}

pub fn execute_fund(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let CounterpartyResponse {
        counterparty,
        other_counterparty,
        storage,
    } = get_counterparty(deps.as_ref(), &info.sender)?;

    let (expected_payment, paid) =
        if let CheckedTokenInfo::Native { amount, denom } = &counterparty.promise {
            let paid = must_pay(&info, denom).map_err(|_| ContractError::InvalidFunds {})?;

            (*amount, paid)
        } else {
            return Err(ContractError::InvalidFunds {});
        };

    do_fund(
        deps,
        counterparty,
        paid,
        expected_payment,
        other_counterparty,
        storage,
    )
}

pub fn execute_withdraw(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let CounterpartyResponse {
        counterparty,
        other_counterparty,
        storage,
    } = get_counterparty(deps.as_ref(), &info.sender)?;

    if !counterparty.provided {
        return Err(ContractError::NoProvision {});
    }

    // The escrow contract completes itself in the same transaction
    // that the second counterparty sends its funds. If that has
    // happens no more withdrawals are allowed. This check isn't
    // strictly needed because the contract won't have enough balance
    // anyhow, but we may as well error nicely.
    if counterparty.provided && other_counterparty.provided {
        return Err(ContractError::Complete {});
    }

    let message = counterparty
        .promise
        .clone()
        .into_send_message(&counterparty.address)?;

    let mut counterparty = counterparty;
    counterparty.provided = false;
    storage.save(deps.storage, &counterparty)?;

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("counterparty", counterparty.address)
        .add_message(message))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Status {} => query_status(deps),
    }
}

pub fn query_status(deps: Deps) -> StdResult<Binary> {
    let counterparty_one = COUNTERPARTY_ONE.load(deps.storage)?;
    let counterparty_two = COUNTERPARTY_TWO.load(deps.storage)?;

    to_json_binary(&StatusResponse {
        counterparty_one,
        counterparty_two,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
