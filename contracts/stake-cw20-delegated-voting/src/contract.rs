use crate::ContractError;
use crate::ContractError::Unauthorized;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::msg::{DelegationResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, Uint256, Uint512, WasmMsg,
};
use cw2::set_contract_version;
use stake_cw20::hooks::StakeChangedHookMsg;
use stake_cw20::msg::StakedBalanceAtHeightResponse;

use crate::msg::QueryMsg::{Delegation, StakedBalanceAtHeight};
use crate::state::{DELEGATIONS, STAKING_CONTRACT, VOTING_POWER};

const CONTRACT_NAME: &str = "crates.io:stake-cw20-external-rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let staking_contract = deps.api.addr_validate(&msg.staking_contract)?;
    STAKING_CONTRACT.save(deps.storage, &staking_contract);

    Ok(Response::new().add_attribute("staking_contract", staking_contract))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info, msg),
        ExecuteMsg::Delegate { address } => Ok(execute_delegate(deps, env, info, address)?),
        ExecuteMsg::Undelegate {} => Ok(execute_undelegate(deps, env, info)?),
    }
}

pub fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response<Empty>, ContractError> {
    let staking_contract = STAKING_CONTRACT.load(deps.storage)?;
    if info.sender != staking_contract {
        return Err(ContractError::Unauthorized {});
    };
    match msg {
        StakeChangedHookMsg::Stake { addr, amount } => execute_stake(deps, env, addr, amount),
        StakeChangedHookMsg::Unstake { addr, amount } => execute_unstake(deps, env, addr, amount),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    addr: Addr,
    amount: Uint128,
) -> Result<Response<Empty>, ContractError> {
    let delegate = DELEGATIONS.may_load(deps.storage, addr)?;
    if let Some(delegate) = delegate {
        let old_voting_power = VOTING_POWER.load(deps.storage, &delegate.clone())?;
        let new_voting_power = old_voting_power + amount;
        VOTING_POWER.save(deps.storage, &delegate, &new_voting_power, env.block.height)?;
    }
    Ok(Response::new().add_attribute("action", "stake"))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    addr: Addr,
    amount: Uint128,
) -> Result<Response<Empty>, ContractError> {
    let delegate = DELEGATIONS.may_load(deps.storage, addr)?;
    if let Some(delegate) = delegate {
        let old_voting_power = VOTING_POWER.load(deps.storage, &delegate.clone())?;
        let new_voting_power = old_voting_power - amount;
        VOTING_POWER.save(deps.storage, &delegate, &new_voting_power, env.block.height)?;
    }
    Ok(Response::new().add_attribute("action", "unstake"))
}

pub fn execute_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response<Empty>, ContractError> {
    let new_delegate = deps.api.addr_validate(&address)?;
    let staked_balance = get_staked_balance(deps.as_ref(), &info.sender)?;
    if staked_balance == Uint128::zero() {
        return Err(ContractError::Unauthorized {});
    }
    let old_delegation = DELEGATIONS.may_load(deps.storage, info.sender.clone())?;
    if let Some(old_delegate) = old_delegation {
        let old_voting_power = VOTING_POWER.load(deps.storage, &old_delegate.clone())?;
        let new_voting_power = old_voting_power - staked_balance;
        VOTING_POWER.save(
            deps.storage,
            &old_delegate,
            &new_voting_power,
            env.block.height,
        )?;
    }
    DELEGATIONS.save(deps.storage, info.sender, &new_delegate);

    let old_voting_power = VOTING_POWER
        .may_load(deps.storage, &new_delegate)?
        .unwrap_or_default();
    let new_voting_power = old_voting_power + staked_balance;
    VOTING_POWER.save(
        deps.storage,
        &new_delegate,
        &new_voting_power,
        env.block.height,
    );

    Ok(Response::new()
        .add_attribute("action", "delegate")
        .add_attribute("delegate", address))
}

pub fn execute_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
    execute_delegate(deps, env, info.clone(), info.sender.into_string())
}

fn get_staked_balance(deps: Deps, addr: &Addr) -> StdResult<Uint128> {
    let staking_contract = STAKING_CONTRACT.load(deps.storage)?;
    let msg = stake_cw20::msg::QueryMsg::StakedBalanceAtHeight {
        address: addr.into(),
        height: None,
    };
    let resp: stake_cw20::msg::StakedBalanceAtHeightResponse =
        deps.querier.query_wasm_smart(staking_contract, &msg)?;
    Ok(resp.balance)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Delegation { address } => to_binary(&get_delegation(deps, address)?),
        QueryMsg::StakedBalanceAtHeight { address, height } => {
            to_binary(&get_staked_balance_at_height(deps, env, address, height)?)
        }
    }
}

pub fn get_delegation(deps: Deps, addr: String) -> StdResult<DelegationResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let delegate = DELEGATIONS
        .may_load(deps.storage, addr.clone())?
        .unwrap_or_else(|| addr);
    Ok(DelegationResponse {
        address: delegate.into_string(),
    })
}

pub fn get_staked_balance_at_height(
    deps: Deps,
    env: Env,
    addr: String,
    height: Option<u64>,
) -> StdResult<StakedBalanceAtHeightResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let height = height.unwrap_or(env.block.height);
    let voting_power = match VOTING_POWER.may_load_at_height(deps.storage, &addr, height)? {
        None => get_staked_balance(deps, &addr)?,
        Some(vp) => vp,
    };
    Ok(StakedBalanceAtHeightResponse {
        balance: voting_power,
        height,
    })
}

#[cfg(test)]
mod tests {}
