use std::ops::Add;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, Uint128, from_binary, Addr};
use cw20::Cw20QueryMsg::Balance;
use cw20::{Cw20CoinVerified, Cw20ReceiveMsg};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse, UnstakingDurationResponse};
use crate::state::{Config, CLAIMS, CONFIG, STAKED_BALANCES, STAKED_TOTAL};
use crate::ContractError;
pub use cw20_base::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
pub use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, execute_update_marketing,
    execute_upload_logo, query_balance, query_download_logo, query_marketing_info, query_minter,
    query_token_info,
};
pub use cw20_base::enumerable::{query_all_accounts, query_all_allowances};
use cw_controllers::ClaimsResponse;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let config = Config {
        token_address: msg.token_address,
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
    }
}
pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.token_address {
       return Err(ContractError::InvalidToken { received: info.sender, expected: config.token_address })
    }
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let api = deps.api;
    match msg {
        ReceiveMsg::Stake{} => {
            execute_stake(deps, env,&api.addr_validate(&wrapper.sender)?, wrapper.amount)
        }
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    STAKED_BALANCES.update(
        deps.storage,
        &sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_add(amount)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> { Ok(total.unwrap_or_default().checked_add(amount)?) },
    )?;

    Ok(Response::new()
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
        .add_attribute("amount", amount)
    )
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
            let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount
            };
            let wasm_msg = cosmwasm_std::WasmMsg::Execute {
                contract_addr: config.token_address.to_string(),
                msg: to_binary(&cw_send_msg)?,
                funds: vec![]
            };
            Ok(Response::new()
                .add_message(wasm_msg)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", "None"))
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
    let config = CONFIG.load(deps.storage)?;
    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: release
    };
    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_binary(&cw_send_msg)?,
        funds: vec![]
    };
    let res = Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "claim")
        .add_attribute("from", info.sender)
        .add_attribute("amount", release);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::StakedBalanceAtHeight { address, height } => to_binary(
            &query_staked_balance_at_height(deps, _env, address, height)?,
        ),
        QueryMsg::TotalStakedAtHeight { height } => {
            to_binary(&query_total_staked_at_height(deps, _env, height)?)
        }
        QueryMsg::UnstakingDuration {} => to_binary(&query_unstaking_duration(deps)?),
        QueryMsg::Claims { address } => to_binary(&query_claims(deps, address)?),
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

pub fn query_claims(deps: Deps, address: String) -> StdResult<ClaimsResponse> {
    CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;
    use std::ops::Add;
    use crate::contract::{
        execute, instantiate, query_staked_balance_at_height, query_total_staked_at_height,
        query_unstaking_duration,
    };
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse, UnstakingDurationResponse};
    use crate::ContractError;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{Addr, Deps, DepsMut, Empty, Env, MessageInfo, OwnedDeps, Response, StdResult, to_binary, Uint128};
    use cw0::Duration;
    use cw20::{Cw20Coin, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};
    use cw20_base::contract::{query_balance, query_minter, query_token_info};
    use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor, next_block};

    use anyhow::Result as AnyResult;
    use crate::msg::QueryMsg::UnstakingDuration;

    const addr1: &str = "addr0001";
    const addr2: &str = "addr0002";
    const addr3: &str = "addr0003";
    const addr4: &str = "addr0004";

    pub fn contract_staking() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        App::default()
    }

    fn get_balance<T: Into<String>, U: Into<String>>(app: &App, contract_addr: T, address: U) -> Uint128 {
        let msg = cw20::Cw20QueryMsg::Balance { address: address.into() };
        let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr,&msg).unwrap();
        result.balance
    }

    fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
        let cw20_id = app.store_code(contract_cw20());
        let msg = cw20_base::msg::InstantiateMsg {
            name: String::from("Test"),
            symbol: String::from("TEST"),
            decimals: 6,
            initial_balances: initial_balances.clone(),
            mint: None,
            marketing: None,
        };

        let contract = app
            .instantiate_contract(cw20_id, Addr::unchecked(addr1), &msg, &[], "cw20", None)
            .unwrap();
        contract
    }

    fn instantiate_staking(
        app: &mut App,
        cw20: Addr,
        unstaking_duration: Option<Duration>,
    ) -> Addr {
        let staking_code_id = app.store_code(contract_staking());
        let msg = crate::msg::InstantiateMsg {
            token_address: cw20,
            unstaking_duration
        };
        app.instantiate_contract(
            staking_code_id,
            Addr::unchecked(addr1),
            &msg,
            &[],
            "staking",
            None,
        ).unwrap()
    }

    fn setup_test_case(
        app: &mut App,
        initial_balances: Vec<Cw20Coin>,
        unstaking_duration: Option<Duration>
    ) -> (Addr, Addr) {
        // 1. Instantiate Gov Token Contract
        let cw20_addr = instantiate_cw20(app, initial_balances);
        app.update_block(next_block);
        let staking_addr = instantiate_staking(
            app,
            cw20_addr.clone(),
            unstaking_duration,
        );
        app.update_block(next_block);
        (staking_addr, cw20_addr)
    }

    fn query_staked_balance<T: Into<String>, U: Into<String>>(app: &App, contract_addr: T, address: U) -> Uint128 {
        let msg = QueryMsg::StakedBalanceAtHeight { address: address.into(), height: None };
        let result: StakedBalanceAtHeightResponse = app.wrap().query_wasm_smart(contract_addr,&msg).unwrap();
        result.balance
    }

    fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
        let msg = QueryMsg::TotalStakedAtHeight { height: None };
        let result: TotalStakedAtHeightResponse = app.wrap().query_wasm_smart(contract_addr,&msg).unwrap();
        result.total
    }

    fn stake_tokens(app: &mut App, staking_addr: &Addr, cw20_addr: &Addr, info: MessageInfo, amount: Uint128) -> AnyResult<AppResponse> {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Stake {}).unwrap()
        };
        app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
    }

    fn unstake_tokens(app: &mut App, staking_addr: &Addr,info: MessageInfo, amount: Uint128) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Unstake {
            amount,
        };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn claim_tokens(app: &mut App, staking_addr: &Addr,info: MessageInfo) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Claim {};
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    #[test]
    fn test_staking() {
        let mut deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let token_address= Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin{ address: addr1.to_string(), amount: amount1 }];
        let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None);

        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();

        // Successful bond
        let amount = Uint128::new(50);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info.clone(), amount).unwrap();
        app.update_block(next_block);
        assert_eq!(
            query_staked_balance(&app, &staking_addr, addr1.to_string()),
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(50u128)
        );
        assert_eq!(
            get_balance(&app, &cw20_addr, addr1.to_string()),
            Uint128::from(50u128)
        );

        // Can't transfer bonded amount
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: addr2.to_string(),
            amount: Uint128::from(51u128),
        };
        let _err = app.borrow_mut().execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[]).unwrap_err();

        // Sucessful transfer
        // Can't transfer bonded amount
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: addr2.to_string(),
            amount: Uint128::from(20u128),
        };
        let res = app.borrow_mut().execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[]).unwrap();

        assert_eq!(
            get_balance(&app, &cw20_addr, addr1.clone()),
            Uint128::from(30u128)
        );
        assert_eq!(
            get_balance(&app, &cw20_addr, addr2.clone()),
            Uint128::from(20u128)
        );

        // Addr 2 successful bond
        let info = mock_info(addr2.as_ref(), &[]);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info.clone(), Uint128::new(20)).unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, addr2.clone()),
            Uint128::from(20u128)
        );
        assert_eq!(
            query_total_staked(&app,&staking_addr),
            Uint128::from(70u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, addr2.clone()), Uint128::zero());

        // Can't unstake when you have more staked
        let info = mock_info(addr2.as_ref(), &[]);
        let _err = unstake_tokens(&mut app,&staking_addr, info,Uint128::new(100)).unwrap_err();

        // Successful unstake
        let info = mock_info(addr2.as_ref(), &[]);
        let res = unstake_tokens(&mut app,&staking_addr, info,Uint128::new(10)).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, addr2),
            Uint128::from(10u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(60u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, addr2), Uint128::from(10u128));

        assert_eq!(
            query_staked_balance(&app, &staking_addr, addr1),
            Uint128::from(50u128)
        );
        assert_eq!(get_balance(&app,&cw20_addr, addr1), Uint128::from(30u128));
    }



       #[test]
       fn test_unstaking_with_claims() {
           let mut deps = mock_dependencies();

           let mut app = mock_app();
           let amount1 = Uint128::from(100u128);
           let unstaking_blocks = 10u64;
           let token_address= Addr::unchecked("token_address");
           let initial_balances = vec![Cw20Coin{ address: addr1.to_string(), amount: amount1 }];
           let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, Some(Duration::Height(unstaking_blocks)));

           let info = mock_info(addr1.as_ref(), &[]);

           // Successful bond
           let res = stake_tokens(&mut app, &staking_addr, &cw20_addr,info,Uint128::new(50)).unwrap();
           app.update_block(next_block);

           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr1.clone()),
               Uint128::from(50u128)
           );
           assert_eq!(
               query_total_staked(&app,&staking_addr),
               Uint128::from(50u128)
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr1),
               Uint128::from(50u128)
           );

           // Unstake
           let info = mock_info(addr1.as_ref(), &[]);
           let res = unstake_tokens(&mut app, &staking_addr, info,Uint128::new(10)).unwrap();
           app.update_block(next_block);

           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr1.clone()),
               Uint128::from(40u128)
           );
           assert_eq!(
               query_total_staked(&app,&staking_addr),
               Uint128::from(40u128)
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr1),
               Uint128::from(50u128)
           );

           // Cannot claim when nothing is available
           let info = mock_info(addr1.as_ref(), &[]);
           let _err: ContractError = claim_tokens(&mut app, &staking_addr, info).unwrap_err().downcast().unwrap();
           assert_eq!(_err, ContractError::NothingToClaim {});

           // Successful claim
           app.update_block(|b| b.height += unstaking_blocks);
           let info = mock_info(addr1.as_ref(), &[]);
           let _res= claim_tokens(&mut app, &staking_addr, info).unwrap();
           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr1.clone()),
               Uint128::from(40u128)
           );
           assert_eq!(
               query_total_staked(&app,&staking_addr),
               Uint128::from(40u128)
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr1),
               Uint128::from(60u128)
           );

           // Unstake and claim multiple
           let info = mock_info(addr1.as_ref(), &[]);
           let info = mock_info(addr1.as_ref(), &[]);
           let res = unstake_tokens(&mut app, &staking_addr, info,Uint128::new(5)).unwrap();
           app.update_block(next_block);

           let info = mock_info(addr1.as_ref(), &[]);
           let info = mock_info(addr1.as_ref(), &[]);
           let res = unstake_tokens(&mut app, &staking_addr, info,Uint128::new(5)).unwrap();
           app.update_block(next_block);

           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr1.clone()),
               Uint128::from(30u128)
           );
           assert_eq!(
               query_total_staked(&app,&staking_addr),
               Uint128::from(30u128)
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr1),
               Uint128::from(60u128)
           );

           app.update_block(|b| b.height += unstaking_blocks);
           let info = mock_info(addr1.as_ref(), &[]);
           let _res= claim_tokens(&mut app, &staking_addr, info).unwrap();
           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr1.clone()),
               Uint128::from(30u128)
           );
           assert_eq!(
               query_total_staked(&app,&staking_addr),
               Uint128::from(30u128)
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr1),
               Uint128::from(70u128)
           );

       }


       #[test]
       fn unstaking_duration_query() {
           let mut app = mock_app();
           let amount1 = Uint128::from(100u128);
           let unstaking_duration = Some(Duration::Height(10));
           let token_address= Addr::unchecked("token_address");
           let initial_balances = vec![Cw20Coin{ address: addr1.to_string(), amount: amount1 }];
           let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, unstaking_duration);

           let msg = QueryMsg::UnstakingDuration {};
           let res: UnstakingDurationResponse = app.borrow_mut().wrap().query_wasm_smart(&staking_addr,&msg).unwrap();
           assert_eq!(
               res.duration,
               unstaking_duration
           );
       }

       #[test]
       fn multiple_address_staking() {
           let amount1 = Uint128::from(100u128);
           let initial_balances = vec![
               Cw20Coin {
                   address: addr1.to_string(),
                   amount: amount1,
               },
               Cw20Coin {
                   address: addr2.to_string(),
                   amount: amount1,
               },
               Cw20Coin {
                   address: addr3.to_string(),
                   amount: amount1,
               },
               Cw20Coin {
                   address: addr4.to_string(),
                   amount: amount1,
               },
           ];
           let mut app = mock_app();
           let amount1 = Uint128::from(100u128);
           let unstaking_blocks = 10u64;
           let token_address= Addr::unchecked("token_address");
           let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, Some(Duration::Height(unstaking_blocks)));

           let info = mock_info(addr1.as_ref(), &[]);
           // Successful bond
           let res = stake_tokens(&mut app,&staking_addr,&cw20_addr,info,amount1).unwrap();
           app.update_block(next_block);

           let info = mock_info(addr2.as_ref(), &[]);
           // Successful bond
           let res = stake_tokens(&mut app,&staking_addr,&cw20_addr,info,amount1).unwrap();
           app.update_block(next_block);

           let info = mock_info(addr3.as_ref(), &[]);
           // Successful bond
           let res = stake_tokens(&mut app,&staking_addr,&cw20_addr,info,amount1).unwrap();
           app.update_block(next_block);

           let info = mock_info(addr4.as_ref(), &[]);
           // Successful bond
           let res = stake_tokens(&mut app,&staking_addr,&cw20_addr,info,amount1).unwrap();
           app.update_block(next_block);

           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr1.clone()),
               amount1
           );
           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr2.clone()),
               amount1
           );
           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr3.clone()),
               amount1
           );
           assert_eq!(
               query_staked_balance(&app,  &staking_addr, addr4.clone()),
               amount1
           );

           assert_eq!(
               query_total_staked(&app,&staking_addr),
               amount1.checked_mul(Uint128::new(4)).unwrap()
           );

           assert_eq!(
               get_balance(&app, &cw20_addr, addr1),
               Uint128::zero()
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr2),
               Uint128::zero()
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr3),
               Uint128::zero()
           );
           assert_eq!(
               get_balance(&app, &cw20_addr, addr4),
               Uint128::zero()
           );

       }

}
