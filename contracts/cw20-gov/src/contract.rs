#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uint128, ensure};
use cw20::Cw20Coin;
use cw20_stakeable::contract::{
    execute_update_marketing, execute_upload_logo, query_balance, query_download_logo,
    query_marketing_info, query_minter, query_token_info, query_allowance, query_all_accounts, query_all_allowances
};
use cw20_stakeable::msg::InstantiateMsg;

use crate::msg::{DelegationResponse, ExecuteMsg, QueryMsg, VotingPowerAtHeightResponse};
use crate::state::{DELEGATIONS, VOTING_POWER};
use cw20_stakeable::ContractError;
use cw20_stakeable::state::STAKED_BALANCES;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(cw20_stakeable::contract::instantiate(deps, _env, _info, msg)?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            cw20_stakeable::contract::execute_transfer(deps, env, info, recipient, amount).map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::Burn { amount } => cw20_stakeable::contract::execute_burn(deps, env, info, amount).map_err(ContractError::Cw20Error),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => cw20_stakeable::contract::execute_send(deps, env, info, contract, amount, msg).map_err(ContractError::Cw20Error),
        ExecuteMsg::Mint { recipient, amount } => cw20_stakeable::contract::execute_mint(deps, env, info, recipient, amount).map_err(ContractError::Cw20Error),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => cw20_stakeable::contract::execute_increase_allowance(deps, env, info, spender, amount, expires).map_err(ContractError::Cw20Error),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => cw20_stakeable::contract::execute_decrease_allowance(deps, env, info, spender, amount, expires).map_err(ContractError::Cw20Error),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => cw20_stakeable::contract::execute_transfer_from(deps, env, info, owner, recipient, amount).map_err(ContractError::Cw20Error),
        ExecuteMsg::BurnFrom { owner, amount } => cw20_stakeable::contract::execute_burn_from(deps, env, info, owner, amount).map_err(ContractError::Cw20Error),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => cw20_stakeable::contract::execute_send_from(deps, env, info, owner, contract, amount, msg).map_err(ContractError::Cw20Error),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => cw20_stakeable::contract::execute_update_marketing(deps, env, info, project, description, marketing).map_err(ContractError::Cw20Error),
        ExecuteMsg::UploadLogo(logo) => cw20_stakeable::contract::execute_upload_logo(deps, env, info, logo).map_err(ContractError::Cw20Error),
        ExecuteMsg::Stake { amount} => execute_stake(deps,env,info,amount),
        ExecuteMsg::Unstake { amount} => execute_unstake(deps,env,info,amount),
        ExecuteMsg::Claim {} => cw20_stakeable::contract::execute_claim(deps,env,info),
        ExecuteMsg::DelegateVotes { recipient } => {
            execute_delegate_votes(deps, env, info, recipient)
        }
    }
}

pub fn execute_stake(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> Result<Response,ContractError> {
    let delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    VOTING_POWER.update(deps.storage, &delegation, env.block.height, |balance: Option<Uint128>| -> StdResult<_> {
        Ok(balance.unwrap_or_default().checked_add(amount)?)
    })?;
    cw20_stakeable::contract::execute_stake(deps,env,info,amount)
}

pub fn execute_unstake(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> Result<Response,ContractError> {
    let delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    VOTING_POWER.update(deps.storage, &delegation, env.block.height, |balance: Option<Uint128>| -> StdResult<_> {
        Ok(balance.unwrap_or_default().checked_sub(amount)?)
    })?;
    cw20_stakeable::contract::execute_unstake(deps,env,info,amount)
}

pub fn execute_delegate_votes(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    let amount = STAKED_BALANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    let old_delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    DELEGATIONS.update(deps.storage, &info.sender, |_| -> StdResult<_> {
        Ok(rcpt_addr.clone())
    })?;
    VOTING_POWER.update(
        deps.storage,
        &old_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    VOTING_POWER.update(
        deps.storage,
        &rcpt_addr,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Custom queries
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query_voting_power_at_height(deps, address, height)?)
        }
        // Inherited from cw20_base
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Delegation { address } => to_binary(&query_delegation(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&query_all_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => to_binary(&query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_binary(&query_download_logo(deps)?),
        QueryMsg::TotalStakedAtHeight { height } => to_binary(&cw20_stakeable::contract::query_total_staked_at_height(deps,_env,height)?),
        QueryMsg::StakedBalanceAtHeight { address, height } => to_binary(&cw20_stakeable::contract::query_staked_balance_at_height(deps,_env,address, height)?),
        QueryMsg::UnstakingDuration {} => to_binary(&cw20_stakeable::contract::query_unstaking_duration(deps)?),
        QueryMsg::Claims { address} => to_binary(&cw20_stakeable::contract::query_claims(deps, address)?),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    address: String,
    height: u64,
) -> StdResult<VotingPowerAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = VOTING_POWER
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(VotingPowerAtHeightResponse { balance, height })
}

pub fn query_delegation(deps: Deps, address: String) -> StdResult<DelegationResponse> {
    let address_addr = deps.api.addr_validate(&address)?;
    let delegation = DELEGATIONS
        .may_load(deps.storage, &address_addr)?
        .unwrap_or(address_addr);
    Ok(DelegationResponse {
        delegation: delegation.into(),
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, CosmosMsg, StdError, SubMsg, WasmMsg};
    use cw20::{BalanceResponse, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};

    use super::*;

    #[test]
    fn test_contract() {
        let mut deps = mock_dependencies();
        let addr = "ADDR0001";
        let delegatee = "ADDR0002";
        let amount = Uint128::new(100);
        let instantiate_msg = InstantiateMsg {
            cw20_base: cw20_base::msg::InstantiateMsg {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: addr.to_string(),
                    amount,
                }],
                mint: None,
                marketing: None,
            },
            unstaking_duration: None
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();
        let res = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(Uint128::zero(), query_voting_power_at_height(deps.as_ref(),addr.to_string(),env.block.height).unwrap().balance);

        // Stake tokens
        let info = mock_info(addr, &[]);
        let res = execute_stake(deps.as_mut(), env.clone(), info, amount).unwrap();
        env.block.height += 1;
        assert_eq!(amount,cw20_stakeable::contract::query_staked_balance_at_height(deps.as_ref(), env.clone(), addr.to_string(), None).unwrap().balance);
        assert_eq!(amount, query_voting_power_at_height(deps.as_ref(),addr.to_string(),env.block.height).unwrap().balance);
        assert_eq!(Uint128::zero(), query_voting_power_at_height(deps.as_ref(),delegatee.to_string(),env.block.height).unwrap().balance);

        // Delegate votes
        let info = mock_info(addr, &[]);
        let res = execute_delegate_votes(deps.as_mut(),env.clone(), info, delegatee.to_string()).unwrap();
        env.block.height += 1;
        assert_eq!(Uint128::zero(), query_voting_power_at_height(deps.as_ref(),addr.to_string(),env.block.height).unwrap().balance);
        assert_eq!(amount, query_voting_power_at_height(deps.as_ref(),delegatee.to_string(),env.block.height).unwrap().balance);

        // Partially unstake
        let info = mock_info(addr, &[]);
        let res = execute_unstake(deps.as_mut(), env.clone(), info, Uint128::new(50)).unwrap();
        env.block.height += 1;
        assert_eq!(Uint128::zero(), query_voting_power_at_height(deps.as_ref(),addr.to_string(),env.block.height).unwrap().balance);
        assert_eq!(Uint128::new(50), query_voting_power_at_height(deps.as_ref(),delegatee.to_string(),env.block.height).unwrap().balance);

        // Fully unstake
        let info = mock_info(addr, &[]);
        let res = execute_unstake(deps.as_mut(), env.clone(), info, Uint128::new(50)).unwrap();
        env.block.height += 1;
        assert_eq!(Uint128::zero(), query_voting_power_at_height(deps.as_ref(),addr.to_string(),env.block.height).unwrap().balance);
        assert_eq!(Uint128::zero(), query_voting_power_at_height(deps.as_ref(),delegatee.to_string(),env.block.height).unwrap().balance);
    }

}
