#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary, Uint128};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TotalPowerResponse, VotingContractResponse};
use crate::state::{CW20_BALANCES, DISTRIBUTION_HEIGHT, NATIVE_BALANCES, TOTAL_POWER, VOTING_CONTRACT};

use dao_interface::voting;

const CONTRACT_NAME: &str = "crates.io:cw-fund-distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store the height
    DISTRIBUTION_HEIGHT.save(deps.storage, &env.block.height)?;

    // validate the contract and save it
    let voting_contract = deps.api.addr_validate(&msg.voting_contract)?;
    VOTING_CONTRACT.save(deps.storage, &voting_contract)?;

    let total_power: voting::TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract.clone(),
        &voting::Query::TotalPowerAtHeight {
            height: Some(env.block.height),
        },
    )?;
    // validate the total power and store it
    if total_power.power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }
    TOTAL_POWER.save(deps.storage, &total_power.power)?;

    Ok(Response::default()
        .add_attribute(
            "distribution_height",
            format!("{}", env.block.height),
        )
        .add_attribute("voting_contract", voting_contract)
        .add_attribute("total_power", total_power.power)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: _,
            amount,
            msg: _,
        }) => execute_fund_cw20(deps, info.sender, amount),
        ExecuteMsg::FundNative {} => execute_fund_native(deps, info),
    }
}

pub fn execute_fund_cw20(
    deps: DepsMut,
    token: Addr,
    amount: Uint128
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::ZeroFunds {});
    }

    let balance = CW20_BALANCES.load(deps.storage, token.clone());
    match balance {
        Ok(old_amount) => CW20_BALANCES.save(
                deps.storage,
                token.clone(),
                &old_amount.checked_add(amount).unwrap(),
            )?,
        Err(_) => CW20_BALANCES.save(
            deps.storage,
            token.clone(),
            &amount,
            )?,
    }

    Ok(Response::default()
        .add_attribute("method", "fund_cw20")
        .add_attribute("token", token)
        .add_attribute("amount", amount)
    )
}

pub fn execute_fund_native(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut response = Response::default()
        .add_attribute("method", "fund_native");

    for Coin {amount, denom } in info.funds.into_iter() {
        match NATIVE_BALANCES.load(deps.storage, denom.clone()) {
            Ok(old_amount) => NATIVE_BALANCES.save(
                deps.storage,
                denom.clone(),
                &old_amount.checked_add(amount).unwrap(),
            ),
            Err(_) => NATIVE_BALANCES.save(
                deps.storage,
                denom.clone(),
                &amount,
            ),
        }.unwrap();
        response = response.add_attribute(denom, amount);
    };

    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingContract {} => query_voting_contract(deps),
        QueryMsg::TotalPower {} => query_total_power(deps),
    }
}

pub fn query_voting_contract(deps: Deps) -> StdResult<Binary> {
    let contract = VOTING_CONTRACT.load(deps.storage)?;
    let distribution_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    to_binary(&VotingContractResponse {
        contract,
        distribution_height,
    })
}

pub fn query_total_power(deps: Deps) -> StdResult<Binary> {
    let total_power  = TOTAL_POWER.load(deps.storage)?;
    to_binary(&TotalPowerResponse {
        total_power,
    })
}
