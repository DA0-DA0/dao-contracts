#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
use crate::msg::{
    ExecuteMsg, GetConfigResponse, GetHooksResponse, InstantiateMsg, QueryMsg, ReceiveMsg,
    StakedBalanceAtHeightResponse, StakedValueResponse, TotalStakedAtHeightResponse,
    TotalValueResponse,
};
use crate::state::{
    Config, BALANCE, CLAIMS, CONFIG, HOOKS, MAX_CLAIMS, STAKED_BALANCES, STAKED_TOTAL,
};
use crate::ContractError;
use cw2::set_contract_version;
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
use cw_utils::Duration;

const CONTRACT_NAME: &str = "crates.io:stake_cw20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let owner = match msg.owner {
        Some(owner) => Some(deps.api.addr_validate(owner.as_str())?),
        None => None,
    };

    let manager = match msg.manager {
        Some(manager) => Some(deps.api.addr_validate(manager.as_str())?),
        None => None,
    };

    let config = Config {
        owner,
        manager,
        token_address: deps.api.addr_validate(&*msg.token_address)?,
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

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
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(info, deps, owner, manager, duration),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
    }
}

pub fn execute_update_config(
    info: MessageInfo,
    deps: DepsMut,
    new_owner: Option<String>,
    new_manager: Option<String>,
    duration: Option<Duration>,
) -> Result<Response, ContractError> {
    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&*new_owner))
        .transpose()?;
    let new_manager = new_manager
        .map(|new_manager| deps.api.addr_validate(&*new_manager))
        .transpose()?;
    let mut config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    };
    if Some(info.sender) != config.owner && new_owner != config.owner {
        return Err(ContractError::OnlyOwnerCanChangeOwner {});
    };

    config.owner = new_owner;
    config.manager = new_manager;

    config.unstaking_duration = duration;

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute(
            "owner",
            config
                .owner
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute(
            "manager",
            config
                .manager
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.token_address {
        return Err(ContractError::InvalidToken {
            received: info.sender,
            expected: config.token_address,
        });
    }
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    match msg {
        ReceiveMsg::Stake {} => execute_stake(deps, env, sender, wrapper.amount),
        ReceiveMsg::Fund {} => execute_fund(deps, env, &sender, wrapper.amount),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let balance = BALANCE.load(deps.storage).unwrap_or_default();
    let staked_total = STAKED_TOTAL.load(deps.storage).unwrap_or_default();
    let amount_to_stake = if staked_total == Uint128::zero() || balance == Uint128::zero() {
        amount
    } else {
        staked_total
            .checked_mul(amount)
            .map_err(StdError::overflow)?
            .checked_div(balance)
            .map_err(StdError::divide_by_zero)?
    };
    STAKED_BALANCES.update(
        deps.storage,
        &sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_add(amount_to_stake)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> {
            Ok(total.unwrap_or_default().checked_add(amount_to_stake)?)
        },
    )?;
    BALANCE.save(
        deps.storage,
        &balance.checked_add(amount).map_err(StdError::overflow)?,
    )?;
    let hook_msgs = stake_hook_msgs(deps.storage, sender.clone(), amount_to_stake)?;
    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
        .add_attribute("amount", amount))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let balance = BALANCE.load(deps.storage).unwrap_or_default();
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    let amount_to_claim = amount
        .checked_mul(balance)
        .map_err(StdError::overflow)?
        .checked_div(staked_total)
        .map_err(StdError::divide_by_zero)?;
    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_sub(amount)?) },
    )?;
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> { Ok(total.unwrap_or_default().checked_sub(amount)?) },
    )?;
    BALANCE.save(
        deps.storage,
        &balance
            .checked_sub(amount_to_claim)
            .map_err(StdError::overflow)?,
    )?;
    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), amount)?;
    match config.unstaking_duration {
        None => {
            let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: amount_to_claim,
            };
            let wasm_msg = cosmwasm_std::WasmMsg::Execute {
                contract_addr: config.token_address.to_string(),
                msg: to_binary(&cw_send_msg)?,
                funds: vec![],
            };
            Ok(Response::new()
                .add_message(wasm_msg)
                .add_submessages(hook_msgs)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", "None"))
        }
        Some(duration) => {
            let outstanding_claims = CLAIMS.query_claims(deps.as_ref(), &info.sender)?.claims;
            if outstanding_claims.len() >= MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            CLAIMS.create_claim(
                deps.storage,
                &info.sender,
                amount_to_claim,
                duration.after(&env.block),
            )?;
            Ok(Response::new()
                .add_attribute("action", "unstake")
                .add_submessages(hook_msgs)
                .add_attribute("from", info.sender)
                .add_attribute("amount", amount)
                .add_attribute("claim_duration", format!("{}", duration)))
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
        amount: release,
    };
    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_binary(&cw_send_msg)?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "claim")
        .add_attribute("from", info.sender)
        .add_attribute("amount", release))
}

pub fn execute_fund(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let balance = BALANCE.load(deps.storage).unwrap_or_default();
    BALANCE.save(
        deps.storage,
        &balance.checked_add(amount).map_err(StdError::overflow)?,
    )?;
    Ok(Response::new()
        .add_attribute("action", "fund")
        .add_attribute("from", sender)
        .add_attribute("amount", amount))
}

pub fn execute_add_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };
    HOOKS.add_hook(deps.storage, addr.clone())?;
    Ok(Response::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };
    HOOKS.remove_hook(deps.storage, addr.clone())?;
    Ok(Response::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::StakedBalanceAtHeight { address, height } => {
            to_binary(&query_staked_balance_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalStakedAtHeight { height } => {
            to_binary(&query_total_staked_at_height(deps, env, height)?)
        }
        QueryMsg::StakedValue { address } => to_binary(&query_staked_value(deps, env, address)?),
        QueryMsg::TotalValue {} => to_binary(&query_total_value(deps, env)?),
        QueryMsg::Claims { address } => to_binary(&query_claims(deps, address)?),
        QueryMsg::GetHooks {} => to_binary(&query_hooks(deps)?),
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

pub fn query_staked_value(
    deps: Deps,
    _env: Env,
    address: String,
) -> StdResult<StakedValueResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCE.load(deps.storage).unwrap_or_default();
    let staked = STAKED_BALANCES
        .load(deps.storage, &address)
        .unwrap_or_default();
    let total = STAKED_TOTAL.load(deps.storage).unwrap_or_default();
    if balance == Uint128::zero() || staked == Uint128::zero() || total == Uint128::zero() {
        Ok(StakedValueResponse {
            value: Uint128::zero(),
        })
    } else {
        let value = staked
            .checked_mul(balance)
            .map_err(StdError::overflow)?
            .checked_div(total)
            .map_err(StdError::divide_by_zero)?;
        Ok(StakedValueResponse { value })
    }
}

pub fn query_total_value(deps: Deps, _env: Env) -> StdResult<TotalValueResponse> {
    let balance = BALANCE.load(deps.storage).unwrap_or_default();
    Ok(TotalValueResponse { total: balance })
}

pub fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(GetConfigResponse {
        owner: config.owner.map(|a| a.to_string()),
        manager: config.manager.map(|a| a.to_string()),
        unstaking_duration: config.unstaking_duration,
        token_address: config.token_address.to_string(),
    })
}

pub fn query_claims(deps: Deps, address: String) -> StdResult<ClaimsResponse> {
    CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;

    use crate::msg::{
        ExecuteMsg, GetConfigResponse, QueryMsg, ReceiveMsg, StakedBalanceAtHeightResponse,
        StakedValueResponse, TotalStakedAtHeightResponse, TotalValueResponse,
    };
    use crate::state::MAX_CLAIMS;
    use crate::ContractError;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_binary, Addr, Empty, MessageInfo, Uint128};
    use cw20::Cw20Coin;
    use cw_utils::Duration;

    use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};

    use anyhow::Result as AnyResult;

    use cw_controllers::{Claim, ClaimsResponse};
    use cw_utils::Expiration::AtHeight;

    const ADDR1: &str = "addr0001";
    const ADDR2: &str = "addr0002";
    const ADDR3: &str = "addr0003";
    const ADDR4: &str = "addr0004";

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

    fn get_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = cw20::Cw20QueryMsg::Balance {
            address: address.into(),
        };
        let result: cw20::BalanceResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
        let cw20_id = app.store_code(contract_cw20());
        let msg = cw20_base::msg::InstantiateMsg {
            name: String::from("Test"),
            symbol: String::from("TEST"),
            decimals: 6,
            initial_balances,
            mint: None,
            marketing: None,
        };

        app.instantiate_contract(cw20_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None)
            .unwrap()
    }

    fn instantiate_staking(
        app: &mut App,
        cw20: Addr,
        unstaking_duration: Option<Duration>,
    ) -> Addr {
        let staking_code_id = app.store_code(contract_staking());
        let msg = crate::msg::InstantiateMsg {
            owner: Some("owner".to_string()),
            manager: Some("manager".to_string()),
            token_address: cw20.to_string(),
            unstaking_duration,
        };
        app.instantiate_contract(
            staking_code_id,
            Addr::unchecked(ADDR1),
            &msg,
            &[],
            "staking",
            None,
        )
        .unwrap()
    }

    fn setup_test_case(
        app: &mut App,
        initial_balances: Vec<Cw20Coin>,
        unstaking_duration: Option<Duration>,
    ) -> (Addr, Addr) {
        // Instantiate cw20 contract
        let cw20_addr = instantiate_cw20(app, initial_balances);
        app.update_block(next_block);
        // Instantiate staking contract
        let staking_addr = instantiate_staking(app, cw20_addr.clone(), unstaking_duration);
        app.update_block(next_block);
        (staking_addr, cw20_addr)
    }

    fn query_staked_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = QueryMsg::StakedBalanceAtHeight {
            address: address.into(),
            height: None,
        };
        let result: StakedBalanceAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> GetConfigResponse {
        let msg = QueryMsg::GetConfig {};
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
    }

    fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
        let msg = QueryMsg::TotalStakedAtHeight { height: None };
        let result: TotalStakedAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.total
    }

    fn query_staked_value<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = QueryMsg::StakedValue {
            address: address.into(),
        };
        let result: StakedValueResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.value
    }

    fn query_total_value<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
        let msg = QueryMsg::TotalValue {};
        let result: TotalValueResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.total
    }

    fn query_claims<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Vec<Claim> {
        let msg = QueryMsg::Claims {
            address: address.into(),
        };
        let result: ClaimsResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.claims
    }

    fn stake_tokens(
        app: &mut App,
        staking_addr: &Addr,
        cw20_addr: &Addr,
        info: MessageInfo,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Stake {}).unwrap(),
        };
        app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
    }

    fn update_config(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
        owner: Option<Addr>,
        manager: Option<Addr>,
        duration: Option<Duration>,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::UpdateConfig {
            owner: owner.map(|a| a.to_string()),
            manager: manager.map(|a| a.to_string()),
            duration,
        };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn unstake_tokens(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Unstake { amount };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn claim_tokens(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Claim {};
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    #[test]
    fn test_update_config() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, _cw20_addr) = setup_test_case(&mut app, initial_balances, None);

        let info = mock_info("owner", &[]);
        let _env = mock_env();
        // Test update admin
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            None,
            Some(Duration::Height(100)),
        )
        .unwrap();

        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.unstaking_duration, Some(Duration::Height(100)));

        // Try updating owner with original owner, which is now invalid
        let info = mock_info("owner", &[]);
        let _err = update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner3")),
            None,
            Some(Duration::Height(100)),
        )
        .unwrap_err();

        // Add manager
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            Some(Addr::unchecked("manager")),
            Some(Duration::Height(100)),
        )
        .unwrap();

        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.manager, Some("manager".to_string()));

        // Manager can update unstaking duration
        let info = mock_info("manager", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            Some(Addr::unchecked("manager")),
            Some(Duration::Height(50)),
        )
        .unwrap();
        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.unstaking_duration, Some(Duration::Height(50)));

        // Manager cannot update owner
        let info = mock_info("manager", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("manager")),
            Some(Addr::unchecked("manager")),
            Some(Duration::Height(50)),
        )
        .unwrap_err();

        // Manager can update manager
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            None,
            Some(Duration::Height(50)),
        )
        .unwrap();

        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.manager, None);

        // Remove owner
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            None,
            None,
            Some(Duration::Height(100)),
        )
        .unwrap();

        // Assert no further updates can be made
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        let err: ContractError = update_config(
            &mut app,
            &staking_addr,
            info,
            None,
            None,
            Some(Duration::Height(100)),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
        assert_eq!(err, ContractError::Unauthorized {});

        let info = mock_info("manager", &[]);
        let _env = mock_env();
        let err: ContractError = update_config(
            &mut app,
            &staking_addr,
            info,
            None,
            None,
            Some(Duration::Height(100)),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_staking() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None);

        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();

        // Successful bond
        let amount = Uint128::new(50);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info.clone(), amount).unwrap();

        // Very important that this balances is not reflected until
        // the next block. This protects us from flash loan hostile
        // takeovers.
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::zero()
        );

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(50u128)
        );
        assert_eq!(
            get_balance(&app, &cw20_addr, ADDR1.to_string()),
            Uint128::from(50u128)
        );

        // Can't transfer bonded amount
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: ADDR2.to_string(),
            amount: Uint128::from(51u128),
        };
        let _err = app
            .borrow_mut()
            .execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
            .unwrap_err();

        // Sucessful transfer of unbonded amount
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: ADDR2.to_string(),
            amount: Uint128::from(20u128),
        };
        let _res = app
            .borrow_mut()
            .execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
            .unwrap();

        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(30u128));
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::from(20u128));

        // Addr 2 successful bond
        let info = mock_info(ADDR2, &[]);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(20)).unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(20u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(70u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::zero());

        // Can't unstake more than you have staked
        let info = mock_info(ADDR2, &[]);
        let _err = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(100)).unwrap_err();

        // Successful unstake
        let info = mock_info(ADDR2, &[]);
        let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(10)).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(10u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(60u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::from(10u128));

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(50u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(30u128));
    }

    #[test]
    fn text_max_claims() {
        let mut app = mock_app();
        let amount1 = Uint128::from(MAX_CLAIMS + 1);
        let unstaking_blocks = 1u64;
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr) = setup_test_case(
            &mut app,
            initial_balances,
            Some(Duration::Height(unstaking_blocks)),
        );

        let info = mock_info(ADDR1, &[]);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info.clone(), amount1).unwrap();

        // Create the max number of claims
        for _ in 0..MAX_CLAIMS {
            unstake_tokens(&mut app, &staking_addr, info.clone(), Uint128::new(1)).unwrap();
        }

        // Additional unstaking attempts ought to fail.
        unstake_tokens(&mut app, &staking_addr, info.clone(), Uint128::new(1)).unwrap_err();

        // Clear out the claims list.
        app.update_block(next_block);
        claim_tokens(&mut app, &staking_addr, info.clone()).unwrap();

        // Unstaking now allowed again.
        unstake_tokens(&mut app, &staking_addr, info.clone(), Uint128::new(1)).unwrap();
        app.update_block(next_block);
        claim_tokens(&mut app, &staking_addr, info).unwrap();

        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), amount1);
    }

    #[test]
    fn test_unstaking_with_claims() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let unstaking_blocks = 10u64;
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr) = setup_test_case(
            &mut app,
            initial_balances,
            Some(Duration::Height(unstaking_blocks)),
        );

        let info = mock_info(ADDR1, &[]);

        // Successful bond
        let _res =
            stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(50)).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(50u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(50u128));

        // Unstake
        let info = mock_info(ADDR1, &[]);
        let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(10)).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(40u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(40u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(50u128));

        // Cannot claim when nothing is available
        let info = mock_info(ADDR1, &[]);
        let _err: ContractError = claim_tokens(&mut app, &staking_addr, info)
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(_err, ContractError::NothingToClaim {});

        // Successful claim
        app.update_block(|b| b.height += unstaking_blocks);
        let info = mock_info(ADDR1, &[]);
        let _res = claim_tokens(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(40u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(40u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(60u128));

        // Unstake and claim multiple
        let _info = mock_info(ADDR1, &[]);
        let info = mock_info(ADDR1, &[]);
        let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(5)).unwrap();
        app.update_block(next_block);

        let _info = mock_info(ADDR1, &[]);
        let info = mock_info(ADDR1, &[]);
        let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(5)).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(30u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(30u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(60u128));

        app.update_block(|b| b.height += unstaking_blocks);
        let info = mock_info(ADDR1, &[]);
        let _res = claim_tokens(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(30u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(30u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(70u128));
    }

    #[test]
    fn multiple_address_staking() {
        let amount1 = Uint128::from(100u128);
        let initial_balances = vec![
            Cw20Coin {
                address: ADDR1.to_string(),
                amount: amount1,
            },
            Cw20Coin {
                address: ADDR2.to_string(),
                amount: amount1,
            },
            Cw20Coin {
                address: ADDR3.to_string(),
                amount: amount1,
            },
            Cw20Coin {
                address: ADDR4.to_string(),
                amount: amount1,
            },
        ];
        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let unstaking_blocks = 10u64;
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw20_addr) = setup_test_case(
            &mut app,
            initial_balances,
            Some(Duration::Height(unstaking_blocks)),
        );

        let info = mock_info(ADDR1, &[]);
        // Successful bond
        let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
        app.update_block(next_block);

        let info = mock_info(ADDR2, &[]);
        // Successful bond
        let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
        app.update_block(next_block);

        let info = mock_info(ADDR3, &[]);
        // Successful bond
        let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
        app.update_block(next_block);

        let info = mock_info(ADDR4, &[]);
        // Successful bond
        let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
        app.update_block(next_block);

        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR1), amount1);
        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR2), amount1);
        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR3), amount1);
        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR4), amount1);

        assert_eq!(
            query_total_staked(&app, &staking_addr),
            amount1.checked_mul(Uint128::new(4)).unwrap()
        );

        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::zero());
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::zero());
        assert_eq!(get_balance(&app, &cw20_addr, ADDR3), Uint128::zero());
        assert_eq!(get_balance(&app, &cw20_addr, ADDR4), Uint128::zero());
    }

    #[test]
    fn test_auto_compounding_staking() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(1000u128);
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None);

        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();

        // Successful bond
        let amount = Uint128::new(100);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();
        app.update_block(next_block);
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_staked_value(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_total_value(&app, &staking_addr),
            Uint128::from(100u128)
        );
        assert_eq!(
            get_balance(&app, &cw20_addr, ADDR1.to_string()),
            Uint128::from(900u128)
        );

        // Add compounding rewards
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
        };
        let _res = app
            .borrow_mut()
            .execute_contract(Addr::unchecked(ADDR1), cw20_addr.clone(), &msg, &[])
            .unwrap();
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_staked_value(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(200u128)
        );
        assert_eq!(
            query_total_value(&app, &staking_addr),
            Uint128::from(200u128)
        );
        assert_eq!(
            get_balance(&app, &cw20_addr, ADDR1.to_string()),
            Uint128::from(800u128)
        );

        // Sucessful transfer of unbonded amount
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: ADDR2.to_string(),
            amount: Uint128::from(100u128),
        };
        let _res = app
            .borrow_mut()
            .execute_contract(Addr::unchecked(ADDR1), cw20_addr.clone(), &msg, &[])
            .unwrap();

        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(700u128));
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::from(100u128));

        // Addr 2 successful bond
        let info = mock_info(ADDR2, &[]);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(100)).unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(150u128)
        );
        assert_eq!(
            query_staked_value(&app, &staking_addr, ADDR2.to_string()),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_total_value(&app, &staking_addr),
            Uint128::from(300u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::zero());

        // Can't unstake more than you have staked
        let info = mock_info(ADDR2, &[]);
        let _err = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(51)).unwrap_err();

        // Add compounding rewards
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: Uint128::from(90u128),
            msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
        };
        let _res = app
            .borrow_mut()
            .execute_contract(Addr::unchecked(ADDR1), cw20_addr.clone(), &msg, &[])
            .unwrap();

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(50u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(150u128)
        );
        assert_eq!(
            query_staked_value(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(260u128)
        );
        assert_eq!(
            query_staked_value(&app, &staking_addr, ADDR2.to_string()),
            Uint128::from(130u128)
        );
        assert_eq!(
            query_total_value(&app, &staking_addr),
            Uint128::from(390u128)
        );
        assert_eq!(
            get_balance(&app, &cw20_addr, ADDR1.to_string()),
            Uint128::from(610u128)
        );

        // Successful unstake
        let info = mock_info(ADDR2, &[]);
        let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(25)).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(25u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(125u128)
        );
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::from(65u128));
    }

    #[test]
    fn test_simple_unstaking_with_duration() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![
            Cw20Coin {
                address: ADDR1.to_string(),
                amount: amount1,
            },
            Cw20Coin {
                address: ADDR2.to_string(),
                amount: amount1,
            },
        ];
        let (staking_addr, cw20_addr) =
            setup_test_case(&mut app, initial_balances, Some(Duration::Height(1)));

        // Bond Address 1
        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        let amount = Uint128::new(100);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();

        // Bond Address 2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        let amount = Uint128::new(100);
        stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();
        app.update_block(next_block);
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(100u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(100u128)
        );

        // Unstake Addr1
        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        let amount = Uint128::new(100);
        unstake_tokens(&mut app, &staking_addr, info, amount).unwrap();

        // Unstake Addr2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        let amount = Uint128::new(100);
        unstake_tokens(&mut app, &staking_addr, info, amount).unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(0u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2.to_string()),
            Uint128::from(0u128)
        );

        // Claim
        assert_eq!(
            query_claims(&app, &staking_addr, ADDR1),
            vec![Claim {
                amount: Uint128::new(100),
                release_at: AtHeight(12349)
            }]
        );
        assert_eq!(
            query_claims(&app, &staking_addr, ADDR2),
            vec![Claim {
                amount: Uint128::new(100),
                release_at: AtHeight(12349)
            }]
        );

        let info = mock_info(ADDR1, &[]);
        claim_tokens(&mut app, &staking_addr, info).unwrap();
        assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(100u128));

        let info = mock_info(ADDR2, &[]);
        claim_tokens(&mut app, &staking_addr, info).unwrap();
        assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::from(100u128));
    }
}
