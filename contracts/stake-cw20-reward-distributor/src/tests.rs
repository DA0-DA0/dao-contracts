use crate::{
    msg::{ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg},
    state::Config,
    ContractError,
};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

const OWNER: &str = "owner";
const OWNER2: &str = "owner2";
const MANAGER: &str = "manager";

pub fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn staking_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        stake_cw20::contract::execute,
        stake_cw20::contract::instantiate,
        stake_cw20::contract::query,
    );
    Box::new(contract)
}

fn distributor_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
    let cw20_id = app.store_code(cw20_contract());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test"),
        symbol: String::from("TEST"),
        decimals: 6,
        initial_balances,
        mint: None,
        marketing: None,
    };

    app.instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None)
        .unwrap()
}

fn instantiate_staking(app: &mut App, cw20_addr: Addr) -> Addr {
    let staking_id = app.store_code(staking_contract());
    let msg = stake_cw20::msg::InstantiateMsg {
        owner: Some(OWNER.to_string()),
        manager: Some(MANAGER.to_string()),
        token_address: cw20_addr.to_string(),
        unstaking_duration: None,
    };
    app.instantiate_contract(
        staking_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        "staking",
        None,
    )
    .unwrap()
}

fn instantiate_distributor(app: &mut App, msg: InstantiateMsg) -> Addr {
    let code_id = app.store_code(distributor_contract());
    app.instantiate_contract(
        code_id,
        Addr::unchecked(OWNER), // TODO: Remove this?
        &msg,
        &[],
        "distributor",
        None,
    )
    .unwrap()
}

fn get_balance_cw20<T: Into<String>, U: Into<String>>(
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

fn get_info<T: Into<String>>(app: &App, distributor_addr: T) -> InfoResponse {
    let result: InfoResponse = app
        .wrap()
        .query_wasm_smart(distributor_addr, &QueryMsg::Info {})
        .unwrap();
    result
}

#[test]
fn test_instantiate() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(&mut app, vec![]);
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: OWNER.to_string(),
        recipient: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };

    let distributor_addr = instantiate_distributor(&mut app, msg);
    let response: InfoResponse = app
        .wrap()
        .query_wasm_smart(&distributor_addr, &QueryMsg::Info {})
        .unwrap();

    assert_eq!(
        response.config,
        Config {
            owner: Addr::unchecked(OWNER),
            recipient: staking_addr,
            reward_rate: Uint128::new(1),
            reward_token: cw20_addr,
        }
    );
    assert_eq!(response.last_payment_block, app.block_info().height);
}

#[test]
fn test_update_config() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(&mut app, vec![]);
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: OWNER.to_string(),
        recipient: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER2.to_string(),
        recipient: staking_addr.to_string(),
        reward_rate: Uint128::new(5),
        reward_token: cw20_addr.to_string(),
    };

    app.execute_contract(Addr::unchecked(OWNER), distributor_addr.clone(), &msg, &[])
        .unwrap();

    let response: InfoResponse = app
        .wrap()
        .query_wasm_smart(&distributor_addr, &QueryMsg::Info {})
        .unwrap();

    assert_eq!(
        response.config,
        Config {
            owner: Addr::unchecked(OWNER2),
            recipient: staking_addr.clone(),
            reward_rate: Uint128::new(5),
            reward_token: cw20_addr.clone(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER2.to_string(),
        recipient: staking_addr.to_string(),
        reward_rate: Uint128::new(7),
        reward_token: cw20_addr.to_string(),
    };

    let err: ContractError = app
        .execute_contract(Addr::unchecked(OWNER), distributor_addr, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_distribute() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: OWNER.to_string(),
        recipient: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: distributor_addr.to_string(),
        amount: Uint128::from(1000u128),
    };
    app.execute_contract(Addr::unchecked(OWNER), cw20_addr.clone(), &msg, &[])
        .unwrap();

    app.update_block(|mut block| block.height += 10);
    app.execute_contract(
        Addr::unchecked(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr.clone());
    assert_eq!(staking_balance, Uint128::new(10));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(990));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);

    app.update_block(|mut block| block.height += 500);
    app.execute_contract(
        Addr::unchecked(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr.clone());
    assert_eq!(staking_balance, Uint128::new(510));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(490));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);

    app.update_block(|mut block| block.height += 1000);
    app.execute_contract(
        Addr::unchecked(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr, staking_addr);
    assert_eq!(staking_balance, Uint128::new(1000));

    let distributor_info = get_info(&app, distributor_addr);
    assert_eq!(distributor_info.balance, Uint128::new(0));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);
}

#[test]
fn test_invalid_addrs() {
    let mut app = App::default();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: OWNER.to_string(),
        recipient: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: "invalid_cw20".to_string(),
    };

    let code_id = app.store_code(distributor_contract());
    let err: ContractError = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "distributor",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidCw20 {});

    let msg = InstantiateMsg {
        owner: OWNER.to_string(),
        recipient: "invalid_staking".to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let err: ContractError = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "distributor",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidStakingContract {});
}
