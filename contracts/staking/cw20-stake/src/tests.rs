use anyhow::Result as AnyResult;
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{to_json_binary, Addr, MessageInfo, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_controllers::{Claim, ClaimsResponse};
use cw_multi_test::{next_block, App, AppResponse, Executor};
use cw_ownable::{Action, Ownership, OwnershipError};
use cw_utils::Duration;
use cw_utils::Expiration::AtHeight;
use dao_testing::contracts::{cw20_base_contract, cw20_stake_contract, v1::cw20_stake_v1_contract};
use dao_voting::duration::UnstakingDurationError;
use std::borrow::BorrowMut;

use crate::msg::{
    ExecuteMsg, ListStakersResponse, MigrateMsg, QueryMsg, ReceiveMsg,
    StakedBalanceAtHeightResponse, StakedValueResponse, StakerBalanceResponse,
    TotalStakedAtHeightResponse, TotalValueResponse,
};
use crate::state::{Config, MAX_CLAIMS};
use cw20_stake::ContractError;

use cw20_stake_v1 as v1;

const ADDR1: &str = "cosmwasm1wtqa75mkgwgncx8v4dep5aygmnq7gspaufggc5ev3u68et43qxmsqy5haw";
const ADDR2: &str = "cosmwasm1g807u64s6uvk3daw4k4h778h850put0qdny3llp3xn43y5dar0hqfdcpt4";
const ADDR3: &str = "cosmwasm137w6v7aa8qvk4mtdh4af9yadvp20yy26pgzy3t2shj6yrwsdtmnscg2cf5";
const ADDR4: &str = "cosmwasm1a0n9l2dvsy6mpkqlx30tgrh4dg74p6ck7465seqh97jall63tdps6j5gwg";
const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

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
    let cw20_id = app.store_code(cw20_base_contract());
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
    let staking_code_id = app.store_code(cw20_stake_contract());
    let msg = crate::msg::InstantiateMsg {
        owner: Some(OWNER.to_string()),
        token_address: cw20.to_string(),
        unstaking_duration,
    };
    app.instantiate_contract(
        staking_code_id,
        Addr::unchecked(ADDR1),
        &msg,
        &[],
        "staking",
        Some("cosmwasm1335hded4gyzpt00fpz75mms4m7ck02wgw07yhw9grahj4dzg4yvqysvwql".to_string()),
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

fn query_owner<T: Into<String>>(app: &App, contract: T) -> Ownership<Addr> {
    app.wrap()
        .query_wasm_smart(contract, &QueryMsg::Ownership {})
        .unwrap()
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
        msg: to_json_binary(&ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
}

fn update_config(
    app: &mut App,
    staking_addr: &Addr,
    info: MessageInfo,
    duration: Option<Duration>,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig { duration };
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
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (_staking_addr, _cw20_addr) =
        setup_test_case(&mut app, initial_balances, Some(Duration::Height(0)));
}

#[test]
#[should_panic(expected = "Provided cw20 errored in response to TokenInfo query")]
fn test_instantiate_with_non_cw20_token() {
    let app = &mut mock_app();
    instantiate_staking(
        app,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        None,
    );
}

#[test]
fn test_update_config() {
    let mut app = mock_app();
    let amount1 = Uint128::from(100u128);
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, _cw20_addr) = setup_test_case(&mut app, initial_balances, None);

    // Owner can update configuration.
    let info = message_info(&Addr::unchecked(OWNER), &[]);
    update_config(&mut app, &staking_addr, info, Some(Duration::Height(1234))).unwrap();
    let config = query_config(&app, &staking_addr);
    assert_eq!(config.unstaking_duration, Some(Duration::Height(1234)));

    // Non owner may not update configuration.
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    let err: ContractError = update_config(&mut app, &staking_addr, info, None)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));

    // Zero durations not allowed.
    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let err: ContractError =
        update_config(&mut app, &staking_addr, info, Some(Duration::Height(0)))
            .unwrap_err()
            .downcast()
            .unwrap();
    assert_eq!(
        err,
        ContractError::UnstakingDurationError(UnstakingDurationError::InvalidUnstakingDuration {})
    );

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let err: ContractError = update_config(&mut app, &staking_addr, info, Some(Duration::Time(0)))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::UnstakingDurationError(UnstakingDurationError::InvalidUnstakingDuration {})
    );
}

#[test]
fn test_staking() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let amount1 = Uint128::from(100u128);
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None);

    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
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
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
    let _err = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(100)).unwrap_err();

    // Successful unstake
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
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
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        initial_balances,
        Some(Duration::Height(unstaking_blocks)),
    );

    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        initial_balances,
        Some(Duration::Height(unstaking_blocks)),
    );

    let info = message_info(&Addr::unchecked(ADDR1), &[]);

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
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    let _err: ContractError = claim_tokens(&mut app, &staking_addr, info)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(_err, ContractError::NothingToClaim {});

    // Successful claim
    app.update_block(|b| b.height += unstaking_blocks);
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
    let _info = message_info(&Addr::unchecked(ADDR1), &[]);
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(5)).unwrap();
    app.update_block(next_block);

    let _info = message_info(&Addr::unchecked(ADDR1), &[]);
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        initial_balances,
        Some(Duration::Height(unstaking_blocks)),
    );

    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    // Successful bond
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    let info = message_info(&Addr::unchecked(ADDR2), &[]);
    // Successful bond
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    let info = message_info(&Addr::unchecked(ADDR3), &[]);
    // Successful bond
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    let info = message_info(&Addr::unchecked(ADDR4), &[]);
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
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None);

    let info = message_info(&Addr::unchecked(ADDR1), &[]);
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
        msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
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
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
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
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
    let _err = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(51)).unwrap_err();

    // Add compounding rewards
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount: Uint128::from(90u128),
        msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
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
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
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
    let _token_address =
        Addr::unchecked("cosmwasm18lu94juelnkcmrkdcy9s46889wc6lqfgwdp8eh2w9fhtrn09j07sfyrn8x");
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
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    let _env = mock_env();
    let amount = Uint128::new(100);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();

    // Bond Address 2
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
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
    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    let _env = mock_env();
    let amount = Uint128::new(100);
    unstake_tokens(&mut app, &staking_addr, info, amount).unwrap();

    // Unstake Addr2
    let info = message_info(&Addr::unchecked(ADDR2), &[]);
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

    let info = message_info(&Addr::unchecked(ADDR1), &[]);
    claim_tokens(&mut app, &staking_addr, info).unwrap();
    assert_eq!(get_balance(&app, &cw20_addr, ADDR1), Uint128::from(100u128));

    let info = message_info(&Addr::unchecked(ADDR2), &[]);
    claim_tokens(&mut app, &staking_addr, info).unwrap();
    assert_eq!(get_balance(&app, &cw20_addr, ADDR2), Uint128::from(100u128));
}

#[test]
fn test_double_unstake_at_height() {
    let mut app = App::default();

    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        vec![Cw20Coin {
            address: "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"
                .to_string(),
            amount: Uint128::new(10),
        }],
        None,
    );

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(
            &Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
            &[],
        ),
        Uint128::new(10),
    )
    .unwrap();

    app.update_block(next_block);

    unstake_tokens(
        &mut app,
        &staking_addr,
        message_info(
            &Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
            &[],
        ),
        Uint128::new(1),
    )
    .unwrap();

    unstake_tokens(
        &mut app,
        &staking_addr,
        message_info(
            &Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
            &[],
        ),
        Uint128::new(9),
    )
    .unwrap();

    app.update_block(next_block);

    // Unstaked balances are not reflected until the following
    // block. Same behavior as staked balances. This is important
    // because otherwise weird things could happen like:
    //
    // 1. I create a proposal (and am allowed to because I have a
    //    staked balance)
    // 2. I unstake all my tokens in the same block.
    //
    // Now there is some strangeness as for part of the block I had a
    // staked balance and was allowed to take actions as if I did, and
    // part of it I did not.
    let balance: StakedBalanceAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr.clone(),
            &QueryMsg::StakedBalanceAtHeight {
                address: "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"
                    .to_string(),
                height: Some(app.block_info().height - 1),
            },
        )
        .unwrap();

    assert_eq!(balance.balance, Uint128::new(10));

    let balance: StakedBalanceAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr,
            &QueryMsg::StakedBalanceAtHeight {
                address: "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"
                    .to_string(),
                height: Some(app.block_info().height),
            },
        )
        .unwrap();

    assert_eq!(balance.balance, Uint128::zero())
}

#[test]
#[ignore = "cw-2: needs test-design refactor (placeholder addresses / cw-multi-test 0.20 contractN naming / dynamic format!() addresses / cw-multi-test 2.x unimplemented features)"]
fn test_query_list_stakers() {
    let mut app = App::default();

    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        vec![
            Cw20Coin {
                address: "cosmwasm18efds2y9je6aywd6e8kcgg7v5zj8hfj89d55rh39nz5pfk7up9gs7vsskd"
                    .to_string(),
                amount: Uint128::new(10),
            },
            Cw20Coin {
                address: "cosmwasm1s2j8n93fgfnsa7fd7x9sarcqgg2zq99ytyhp7td9zvrce39w6las33q0tj"
                    .to_string(),
                amount: Uint128::new(20),
            },
            Cw20Coin {
                address: "cosmwasm1nal6cdctjk63cuv3hxsnvku23frwy368flzcq6h05pxp8wucka4sfn3ltg"
                    .to_string(),
                amount: Uint128::new(30),
            },
            Cw20Coin {
                address: "cosmwasm1eksjat2v5r4syyyv2esrh4hcuw8fumfnccy2w30r5jasm2zfyknq33xspn"
                    .to_string(),
                amount: Uint128::new(40),
            },
        ],
        None,
    );

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(
            &Addr::unchecked("cosmwasm18efds2y9je6aywd6e8kcgg7v5zj8hfj89d55rh39nz5pfk7up9gs7vsskd"),
            &[],
        ),
        Uint128::new(10),
    )
    .unwrap();

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(
            &Addr::unchecked("cosmwasm1s2j8n93fgfnsa7fd7x9sarcqgg2zq99ytyhp7td9zvrce39w6las33q0tj"),
            &[],
        ),
        Uint128::new(20),
    )
    .unwrap();

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(
            &Addr::unchecked("cosmwasm1nal6cdctjk63cuv3hxsnvku23frwy368flzcq6h05pxp8wucka4sfn3ltg"),
            &[],
        ),
        Uint128::new(30),
    )
    .unwrap();

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(
            &Addr::unchecked("cosmwasm1eksjat2v5r4syyyv2esrh4hcuw8fumfnccy2w30r5jasm2zfyknq33xspn"),
            &[],
        ),
        Uint128::new(40),
    )
    .unwrap();

    // check first 2
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr.clone(),
            &QueryMsg::ListStakers {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();

    let test_res = ListStakersResponse {
        stakers: vec![
            StakerBalanceResponse {
                address: "cosmwasm18efds2y9je6aywd6e8kcgg7v5zj8hfj89d55rh39nz5pfk7up9gs7vsskd"
                    .to_string(),
                balance: Uint128::new(10),
            },
            StakerBalanceResponse {
                address: "cosmwasm1s2j8n93fgfnsa7fd7x9sarcqgg2zq99ytyhp7td9zvrce39w6las33q0tj"
                    .to_string(),
                balance: Uint128::new(20),
            },
        ],
    };

    assert_eq!(stakers, test_res);

    // skip first and grab 2
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr,
            &QueryMsg::ListStakers {
                start_after: Some("ekez1".to_string()),
                limit: Some(2),
            },
        )
        .unwrap();

    let test_res = ListStakersResponse {
        stakers: vec![
            StakerBalanceResponse {
                address: "cosmwasm1s2j8n93fgfnsa7fd7x9sarcqgg2zq99ytyhp7td9zvrce39w6las33q0tj"
                    .to_string(),
                balance: Uint128::new(20),
            },
            StakerBalanceResponse {
                address: "cosmwasm1nal6cdctjk63cuv3hxsnvku23frwy368flzcq6h05pxp8wucka4sfn3ltg"
                    .to_string(),
                balance: Uint128::new(30),
            },
        ],
    };

    assert_eq!(stakers, test_res)
}

#[test]
fn test_ownership_transfer() {
    let mut app = App::default();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr, None);

    app.execute_contract(
        Addr::unchecked(OWNER),
        staking_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: ADDR1.to_string(),
            expiry: None,
        }),
        &[],
    )
    .unwrap();

    let ownership = query_owner(&app, &staking_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(Addr::unchecked(OWNER)),
            pending_owner: Some(Addr::unchecked(ADDR1)),
            pending_expiry: None
        }
    );

    app.execute_contract(
        Addr::unchecked(ADDR1),
        staking_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership),
        &[],
    )
    .unwrap();

    let ownership = query_owner(&app, &staking_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(Addr::unchecked(ADDR1)),
            pending_owner: None,
            pending_expiry: None
        }
    );
}

#[test]
#[ignore = "V1 migration stubbed for cw-std 2.x — needs Stage 3 storage-bytes shim"]
fn test_migrate_from_v1() {
    let mut app = App::default();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::from(1000u64),
        }],
    );

    let v1_code = app.store_code(cw20_stake_v1_contract());
    let v2_code = app.store_code(cw20_stake_contract());

    let staking = app
        .instantiate_contract(
            v1_code,
            Addr::unchecked(OWNER),
            &v1::msg::InstantiateMsg {
                owner: Some(OWNER.to_string()),
                manager: Some(OWNER.to_string()),
                token_address: cw20_addr.to_string(),
                unstaking_duration: None,
            },
            &[],
            "staking".to_string(),
            Some(OWNER.to_string()),
        )
        .unwrap();

    app.execute(
        Addr::unchecked(OWNER),
        WasmMsg::Migrate {
            contract_addr: staking.to_string(),
            new_code_id: v2_code,
            msg: to_json_binary(&MigrateMsg::FromV1 {}).unwrap(),
        }
        .into(),
    )
    .unwrap();

    // can not migrate more than once.
    let err: ContractError = app
        .execute(
            Addr::unchecked(OWNER),
            WasmMsg::Migrate {
                contract_addr: staking.to_string(),
                new_code_id: v2_code,
                msg: to_json_binary(&MigrateMsg::FromV1 {}).unwrap(),
            }
            .into(),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::AlreadyMigrated {});

    // owner is moved into cw_ownable.
    let ownership = query_owner(&app, &staking);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(Addr::unchecked(OWNER)),
            pending_owner: None,
            pending_expiry: None
        }
    );

    // config is loadable and has no manager, but is otherwise
    // unchanged.
    let config = query_config(&app, &staking);
    assert_eq!(
        config,
        Config {
            token_address: cw20_addr,
            unstaking_duration: None,
        }
    );
}
