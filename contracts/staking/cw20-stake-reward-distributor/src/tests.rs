use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{ExecuteMsg, InfoResponse, InstantiateMsg, MigrateMsg, QueryMsg},
    state::Config,
    ContractError,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Empty, Uint128,
};
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
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
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
    let msg = cw20_stake::msg::InstantiateMsg {
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
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };

    let distributor_addr = instantiate_distributor(&mut app, msg);
    let response: InfoResponse = app
        .wrap()
        .query_wasm_smart(distributor_addr, &QueryMsg::Info {})
        .unwrap();

    assert_eq!(
        response.config,
        Config {
            owner: Addr::unchecked(OWNER),
            staking_addr,
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
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER2.to_string(),
        staking_addr: staking_addr.to_string(),
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
            staking_addr: staking_addr.clone(),
            reward_rate: Uint128::new(5),
            reward_token: cw20_addr.clone(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER2.to_string(),
        staking_addr: staking_addr.to_string(),
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
        staking_addr: staking_addr.to_string(),
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

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr.clone());
    assert_eq!(staking_balance, Uint128::new(1000));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(0));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);
    let last_payment_block = distributor_info.last_payment_block;

    // Pays out nothing
    app.update_block(|mut block| block.height += 1100);
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            distributor_addr.clone(),
            &ExecuteMsg::Distribute {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::ZeroRewards {}));

    let staking_balance = get_balance_cw20(&app, cw20_addr, staking_addr);
    assert_eq!(staking_balance, Uint128::new(1000));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(0));
    assert_eq!(distributor_info.last_payment_block, last_payment_block);

    // go to a block before the last payment
    app.update_block(|mut block| block.height -= 2000);
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            distributor_addr,
            &ExecuteMsg::Distribute {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::RewardsDistributedForBlock {}));
}

#[test]
fn test_instantiate_invalid_addrs() {
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
        staking_addr: staking_addr.to_string(),
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
        staking_addr: "invalid_staking".to_string(),
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

#[test]
fn test_update_config_invalid_addrs() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(&mut app, vec![]);
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: OWNER.to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER.to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(5),
        reward_token: "invalid_cw20".to_string(),
    };

    let err: ContractError = app
        .execute_contract(Addr::unchecked(OWNER), distributor_addr.clone(), &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidCw20 {});

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER.to_string(),
        staking_addr: "invalid_staking".to_string(),
        reward_rate: Uint128::new(5),
        reward_token: staking_addr.to_string(),
    };

    let err: ContractError = app
        .execute_contract(Addr::unchecked(OWNER), distributor_addr, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidStakingContract {});
}

#[test]
fn test_withdraw() {
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
        staking_addr: staking_addr.to_string(),
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

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr);
    assert_eq!(staking_balance, Uint128::new(10));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(990));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);

    // Unauthorized user cannot withdraw funds
    let err = app
        .execute_contract(
            Addr::unchecked(MANAGER),
            distributor_addr.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap_err();

    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

    // Withdraw funds
    app.execute_contract(
        Addr::unchecked(OWNER),
        distributor_addr,
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();

    let owner_balance = get_balance_cw20(&app, cw20_addr, Addr::unchecked(OWNER));
    assert_eq!(owner_balance, Uint128::new(990));
}

#[test]
fn test_dao_deploy() {
    // DAOs will deploy this contract with following steps
    // Contract is instantiated by any address with 0 reward rate
    // Dao updates reward rate and funds in same transaction
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
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(0),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    app.update_block(|mut block| block.height += 1000);

    let msg = ExecuteMsg::UpdateConfig {
        owner: OWNER.to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    app.execute_contract(Addr::unchecked(OWNER), distributor_addr.clone(), &msg, &[])
        .unwrap();

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

    let staking_balance = get_balance_cw20(&app, cw20_addr, staking_addr);
    assert_eq!(staking_balance, Uint128::new(10));

    let distributor_info = get_info(&app, distributor_addr);
    assert_eq!(distributor_info.balance, Uint128::new(990));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}
