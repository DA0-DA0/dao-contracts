#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, Uint128,
};

use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, StakedBalanceAtHeightResponse,
    TotalStakedAtHeightResponse, UnstakingDurationResponse,
};
use crate::state::{Config, CLAIMS, CONFIG, STAKED_BALANCES, STAKED_TOTAL};
use crate::ContractError;
use cw20_base::state::BALANCES;
pub use cw20_base::contract::{execute_transfer, execute_burn, execute_mint, execute_send, execute_update_marketing, execute_upload_logo};
pub use cw20_base::allowances::{execute_send_from, execute_transfer_from, execute_burn_from, execute_increase_allowance, execute_decrease_allowance};
use cw_controllers::ClaimsResponse;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let config = Config {
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(cw20_base::contract::instantiate(
        deps,
        _env,
        _info,
        msg.cw20_base,
    )?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, _env, info, recipient, amount)
                .map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::Burn { amount } => execute_burn(deps, _env, info, amount)
            .map_err(ContractError::Cw20Error),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, _env, info, contract, amount, msg)
            .map_err(ContractError::Cw20Error),
        ExecuteMsg::Mint { recipient, amount } => {
            execute_mint(deps, _env, info, recipient, amount)
                .map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(
            deps, _env, info, spender, amount, expires,
        )
        .map_err(ContractError::Cw20Error),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(
            deps, _env, info, spender, amount, expires,
        )
        .map_err(ContractError::Cw20Error),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => {
            execute_transfer_from(deps, _env, info, owner, recipient, amount)
                .map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::BurnFrom { owner, amount } => {
            execute_burn_from(deps, _env, info, owner, amount)
                .map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => {
            execute_send_from(deps, _env, info, owner, contract, amount, msg)
                .map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => execute_update_marketing(
            deps,
            _env,
            info,
            project,
            description,
            marketing,
        )
        .map_err(ContractError::Cw20Error),
        ExecuteMsg::UploadLogo(logo) => {
            execute_upload_logo(deps, _env, info, logo)
                .map_err(ContractError::Cw20Error)
        }
        ExecuteMsg::Stake { amount } => execute_stake(deps, _env, info, amount),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, _env, info, amount),
        ExecuteMsg::Claim {} => execute_claim(deps, _env, info),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    BALANCES.update(deps.storage, &info.sender, |bal| -> StdResult<Uint128> {
        Ok(bal.unwrap_or_default().checked_sub(amount)?)
    })?;
    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        _env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_add(amount)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        _env.block.height,
        |total| -> StdResult<Uint128> { Ok(total.unwrap_or_default().checked_add(amount)?) },
    )?;
    let res = Response::new()
        .add_attribute("action", "stake")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_unstake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        _env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_sub(amount)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        _env.block.height,
        |total| -> StdResult<Uint128> { Ok(total.unwrap_or_default().checked_sub(amount)?) },
    )?;
    match config.unstaking_duration {
        None => {
            BALANCES.update(deps.storage, &info.sender, |bal| -> StdResult<Uint128> {
                Ok(bal.unwrap_or_default().checked_add(amount)?)
            })?;
            let res = Response::new()
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", "None");
            Ok(res)
        }
        Some(duration) => {
            CLAIMS.create_claim(
                deps.storage,
                &info.sender,
                amount,
                duration.after(&_env.block),
            )?;
            let res = Response::new()
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", format!("{}", duration));
            Ok(res)
        }
    }
}

pub fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let release = CLAIMS.claim_tokens(deps.storage, &info.sender, &_env.block, None)?;
    if release.is_zero() {
        return Err(ContractError::NothingToClaim {});
    }
    BALANCES.update(deps.storage, &info.sender, |bal| -> StdResult<Uint128> {
        Ok(bal.unwrap_or_default().checked_add(release)?)
    })?;
    let res = Response::new()
        .add_attribute("action", "claim")
        .add_attribute("from", info.sender)
        .add_attribute("amount", release);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Inherited from cw20_base
        QueryMsg::Balance { address } => {
            to_binary(&cw20_base::contract::query_balance(deps, address)?)
        }
        QueryMsg::TokenInfo {} => to_binary(&cw20_base::contract::query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&cw20_base::contract::query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => to_binary(
            &cw20_base::allowances::query_allowance(deps, owner, spender)?,
        ),
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&cw20_base::enumerable::query_all_allowances(
            deps,
            owner,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => to_binary(
            &cw20_base::enumerable::query_all_accounts(deps, start_after, limit)?,
        ),
        QueryMsg::MarketingInfo {} => to_binary(&cw20_base::contract::query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_binary(&cw20_base::contract::query_download_logo(deps)?),
        QueryMsg::StakedBalanceAtHeight { address, height } => to_binary(
            &query_staked_balance_at_height(deps, _env, address, height)?,
        ),
        QueryMsg::TotalStakedAtHeight { height } => {
            to_binary(&query_total_staked_at_height(deps, _env, height)?)
        }
        QueryMsg::UnstakingDuration {} => to_binary(&query_unstaking_duration(deps)?),
        QueryMsg::Claims { address } => {
            to_binary(&query_claims(deps,address)?)
        }
    }
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    _env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<StakedBalanceAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(_env.block.height);
    let balance = STAKED_BALANCES
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(StakedBalanceAtHeightResponse { balance, height })
}

pub fn query_total_staked_at_height(
    deps: Deps,
    _env: Env,
    height: Option<u64>,
) -> StdResult<TotalStakedAtHeightResponse> {
    let height = height.unwrap_or(_env.block.height);
    let total = STAKED_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(TotalStakedAtHeightResponse { total, height })
}

pub fn query_unstaking_duration(deps: Deps) -> StdResult<UnstakingDurationResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(UnstakingDurationResponse {
        duration: config.unstaking_duration,
    })
}

pub fn query_claims (deps: Deps, address: String) -> StdResult<ClaimsResponse> {
    CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

#[cfg(test)]
mod tests {
    use crate::contract::{
        execute, instantiate, query_staked_balance_at_height, query_total_staked_at_height,
        query_unstaking_duration,
    };
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use crate::ContractError;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Deps, DepsMut, Uint128};
    use cw0::Duration;
    use cw20::{Cw20Coin, MinterResponse, TokenInfoResponse};
    use cw20_base::contract::{query_balance, query_minter, query_token_info};

    fn get_balance<T: Into<String>>(deps: Deps, address: T) -> Uint128 {
        query_balance(deps, address.into()).unwrap().balance
    }

    // this will set up the instantiation for other tests
    fn do_instantiate(
        mut deps: DepsMut,
        _addr: &str,
        initial_balances: Vec<Cw20Coin>,
        mint: Option<MinterResponse>,
        unstaking_duration: Option<Duration>,
    ) -> TokenInfoResponse {
        let instantiate_msg = InstantiateMsg {
            cw20_base: cw20_base::msg::InstantiateMsg {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                initial_balances: initial_balances.clone(),
                mint: mint.clone(),
                marketing: None,
            },
            unstaking_duration,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let meta = query_token_info(deps.as_ref()).unwrap();
        let total = initial_balances.into_iter().map(|x| x.amount).sum();
        assert_eq!(
            meta,
            TokenInfoResponse {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                total_supply: total,
            }
        );
        assert_eq!(query_minter(deps.as_ref()).unwrap(), mint,);
        meta
    }

    #[test]
    fn test_staking() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let amount1 = Uint128::from(100u128);
        let initial_balances = vec![Cw20Coin {
            address: addr1.clone(),
            amount: amount1,
        }];
        do_instantiate(deps.as_mut(), &addr1, initial_balances, None, None);

        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();

        // Can't bond more then you have
        let msg = ExecuteMsg::Stake {
            amount: Uint128::from(101u128),
        };
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Successful bond
        let msg = ExecuteMsg::Stake {
            amount: Uint128::from(50u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(50u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(50u128)
        );

        // Can't bond more then you have
        let msg = ExecuteMsg::Stake {
            amount: Uint128::from(51u128),
        };
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Can't transfer bonded amount
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: Uint128::from(51u128),
        };
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Sucessful transfer
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: Uint128::from(20u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(30u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr2.clone()),
            Uint128::from(20u128)
        );

        // Addr 2 successful bond
        let info = mock_info(addr2.as_ref(), &[]);
        let msg = ExecuteMsg::Stake {
            amount: Uint128::from(20u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr2.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(20u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(70u128)
        );
        assert_eq!(get_balance(deps.as_ref(), addr2.clone()), Uint128::zero());

        // Can't unstake when you have more staked
        let info = mock_info(addr2.as_ref(), &[]);
        let msg = ExecuteMsg::Unstake {
            amount: Uint128::from(100u128),
        };
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Successful unstake
        let info = mock_info(addr2.as_ref(), &[]);
        let msg = ExecuteMsg::Unstake {
            amount: Uint128::from(10u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr2.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(10u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(60u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr2.clone()),
            Uint128::from(10u128)
        );

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(50u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(30u128)
        );
    }

    #[test]
    fn test_unstaking_with_claims() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let amount1 = Uint128::from(100u128);
        let initial_balances = vec![Cw20Coin {
            address: addr1.clone(),
            amount: amount1,
        }];
        let unstaking_blocks = 10u64;
        do_instantiate(
            deps.as_mut(),
            &addr1,
            initial_balances,
            None,
            Some(Duration::Height(unstaking_blocks)),
        );

        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();

        // Successful bond
        let msg = ExecuteMsg::Stake {
            amount: Uint128::from(50u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(50u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(50u128)
        );

        // Unstake
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Unstake {
            amount: Uint128::from(10u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(40u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(40u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(50u128)
        );

        // Cannot claim when nothing is available
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Claim {};
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(_err, ContractError::NothingToClaim {});

        // Successful claim
        env.block.height = env.block.height + unstaking_blocks;
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Claim {};
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(40u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(40u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(60u128)
        );

        // Unstake and claim multiple
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Unstake {
            amount: Uint128::from(5u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Unstake {
            amount: Uint128::from(5u128),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(30u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(30u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(60u128)
        );

        env.block.height = env.block.height + unstaking_blocks;
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Claim {};
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            Uint128::from(30u128)
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            Uint128::from(30u128)
        );
        assert_eq!(
            get_balance(deps.as_ref(), addr1.clone()),
            Uint128::from(70u128)
        );
    }

    #[test]
    fn unstaking_duration_query() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let amount1 = Uint128::from(100u128);
        let initial_balances = vec![Cw20Coin {
            address: addr1.clone(),
            amount: amount1,
        }];
        let unstaking_duration = Some(Duration::Height(10));
        do_instantiate(
            deps.as_mut(),
            &addr1,
            initial_balances,
            None,
            unstaking_duration,
        );
        assert_eq!(
            query_unstaking_duration(deps.as_ref()).unwrap().duration,
            unstaking_duration
        );
    }

    #[test]
    fn multiple_address_staking() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let addr3 = String::from("addr0003");
        let addr4 = String::from("addr0004");
        let amount1 = Uint128::from(100u128);
        let initial_balances = vec![
            Cw20Coin {
                address: addr1.clone(),
                amount: amount1,
            },
            Cw20Coin {
                address: addr2.clone(),
                amount: amount1,
            },
            Cw20Coin {
                address: addr3.clone(),
                amount: amount1,
            },
            Cw20Coin {
                address: addr4.clone(),
                amount: amount1,
            },
        ];
        do_instantiate(deps.as_mut(), &addr1, initial_balances, None, None);
        let mut env = mock_env();

        let info = mock_info(addr1.as_ref(), &[]);
        // Successful bond
        let msg = ExecuteMsg::Stake { amount: amount1 };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        let info = mock_info(addr2.as_ref(), &[]);
        // Successful bond
        let msg = ExecuteMsg::Stake { amount: amount1 };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        let info = mock_info(addr3.as_ref(), &[]);
        // Successful bond
        let msg = ExecuteMsg::Stake { amount: amount1 };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        let info = mock_info(addr4.as_ref(), &[]);
        // Successful bond
        let msg = ExecuteMsg::Stake { amount: amount1 };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        env.block.height = env.block.height + 1;

        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr1.clone(), None)
                .unwrap()
                .balance,
            amount1
        );
        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr2.clone(), None)
                .unwrap()
                .balance,
            amount1
        );
        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr3.clone(), None)
                .unwrap()
                .balance,
            amount1
        );
        assert_eq!(
            query_staked_balance_at_height(deps.as_ref(), env.clone(), addr4.clone(), None)
                .unwrap()
                .balance,
            amount1
        );
        assert_eq!(
            query_total_staked_at_height(deps.as_ref(), env.clone(), None)
                .unwrap()
                .total,
            amount1.checked_mul(Uint128::new(4)).unwrap()
        );
        assert_eq!(get_balance(deps.as_ref(), addr1.clone()), Uint128::zero());
        assert_eq!(get_balance(deps.as_ref(), addr2.clone()), Uint128::zero());
        assert_eq!(get_balance(deps.as_ref(), addr3.clone()), Uint128::zero());
        assert_eq!(get_balance(deps.as_ref(), addr4.clone()), Uint128::zero());
    }
}
