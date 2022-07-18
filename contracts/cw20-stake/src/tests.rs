use std::borrow::BorrowMut;

use crate::msg::{
    ExecuteMsg, QueryMsg, ReceiveMsg, StakedBalanceAtHeightResponse, StakedValueResponse,
    TotalStakedAtHeightResponse, TotalValueResponse,
};
use crate::state::{Config, MAX_CLAIMS};
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
    let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
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

fn instantiate_staking(app: &mut App, cw20: Addr, unstaking_duration: Option<Duration>) -> Addr {
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

fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> Config {
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

fn claim_tokens(app: &mut App, staking_addr: &Addr, info: MessageInfo) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Claim {};
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

#[test]
#[should_panic(expected = "Invalid unstaking duration, unstaking duration cannot be 0")]
fn test_instantiate_invalid_unstaking_duration() {
    let mut app = mock_app();
    let amount1 = Uint128::from(100u128);
    let _token_address = Addr::unchecked("token_address");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (_staking_addr, _cw20_addr) =
        setup_test_case(&mut app, initial_balances, Some(Duration::Height(0)));
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
    assert_eq!(config.owner, Some(Addr::unchecked("owner2")));
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
    assert_eq!(config.owner, Some(Addr::unchecked("owner2")));
    assert_eq!(config.manager, Some(Addr::unchecked("manager")));

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
    assert_eq!(config.owner, Some(Addr::unchecked("owner2")));
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
    assert_eq!(config.owner, Some(Addr::unchecked("owner2")));
    assert_eq!(config.manager, None);

    // Invalid duration
    let info = mock_info("owner2", &[]);
    let _env = mock_env();
    let err: ContractError = update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("owner2")),
        None,
        Some(Duration::Height(0)),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::InvalidUnstakingDuration {});

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
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(50)).unwrap();
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
