use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env},
    to_binary, Addr, Empty, Uint128,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_denom::UncheckedDenom;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw_vesting::{
    msg::{InstantiateMsg as PayrollInstantiateMsg, QueryMsg as PayrollQueryMsg},
    state::{UncheckedVestingParams, VestingPayment, VestingPaymentStatus},
};
use wynd_utils::Curve;

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg},
    state::VestingContract,
    ContractError,
};

const ALICE: &str = "alice";
const BOB: &str = "bob";
const INITIAL_BALANCE: u128 = 1000000000;
const NATIVE_DENOM: &str = "denom";

fn factory_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn cw_vesting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_vesting::contract::execute,
        cw_vesting::contract::instantiate,
        cw_vesting::contract::query,
    );
    Box::new(contract)
}

#[test]
pub fn test_instantiate_native_payroll_contract() {
    let mut app = App::default();
    let code_id = app.store_code(factory_contract());
    let cw_vesting_code_id = app.store_code(cw_vesting_contract());

    // Instantiate factory with only Alice allowed to instantiate payroll contracts
    let instantiate = InstantiateMsg {
        owner: Some(ALICE.to_string()),
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Mint alice and bob native tokens
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: ALICE.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: BOB.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let instantiate_payroll_msg = ExecuteMsg::InstantiateNativePayrollContract {
        instantiate_msg: PayrollInstantiateMsg {
            owner: Some(ALICE.to_string()),
            params: UncheckedVestingParams {
                recipient: BOB.to_string(),
                amount: Uint128::new(1000000),
                denom: unchecked_denom,
                vesting_schedule,
                title: None,
                description: None,
            },
        },
        code_id: cw_vesting_code_id,
        label: "Payroll".to_string(),
    };

    let res = app
        .execute_contract(
            Addr::unchecked(ALICE),
            factory_addr.clone(),
            &instantiate_payroll_msg,
            &coins(amount.into(), NATIVE_DENOM),
        )
        .unwrap();

    // BOB can't instantiate as owner is configured
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(BOB),
            factory_addr.clone(),
            &instantiate_payroll_msg,
            &coins(amount.into(), NATIVE_DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Get the payroll address from the instantiate event
    let instantiate_event = &res.events[2];
    assert_eq!(instantiate_event.ty, "instantiate");
    let cw_vesting_addr = instantiate_event.attributes[0].value.clone();

    // Check that admin of contract is owner specified in Instantiation Message
    let contract_info = app
        .wrap()
        .query_wasm_contract_info(cw_vesting_addr)
        .unwrap();
    assert_eq!(contract_info.admin, Some(ALICE.to_string()));

    // Test query list of contracts
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContracts {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Test query by instantiator
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContractsByInstantiator {
                instantiator: ALICE.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Test query by instantiator with no results
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContractsByInstantiator {
                instantiator: BOB.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 0);

    // Test query by recipient
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContractsByRecipient {
                recipient: BOB.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Test query by recipient no results
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr,
            &QueryMsg::ListVestingContractsByRecipient {
                recipient: ALICE.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 0);
}

#[test]
pub fn test_instantiate_cw20_payroll_contract() {
    let mut app = App::default();
    let code_id = app.store_code(factory_contract());
    let cw20_code_id = app.store_code(cw20_contract());
    let cw_vesting_code_id = app.store_code(cw_vesting_contract());

    // Instantiate cw20 contract with balances for Alice
    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(ALICE),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: ALICE.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    let instantiate = InstantiateMsg {
        owner: Some(ALICE.to_string()),
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Mint alice native tokens
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: ALICE.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Cw20(cw20_addr.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let instantiate_payroll_msg = PayrollInstantiateMsg {
        owner: Some(ALICE.to_string()),
        params: UncheckedVestingParams {
            recipient: BOB.to_string(),
            amount: Uint128::new(1000000),
            denom: unchecked_denom,
            vesting_schedule,
            title: None,
            description: None,
        },
    };

    // Attempting to call InstantiatePayrollContract directly with cw20 fails
    app.execute_contract(
        Addr::unchecked(ALICE),
        factory_addr.clone(),
        &ExecuteMsg::InstantiateNativePayrollContract {
            instantiate_msg: instantiate_payroll_msg.clone(),
            code_id: cw_vesting_code_id,
            label: "Payroll".to_string(),
        },
        &coins(amount.into(), NATIVE_DENOM),
    )
    .unwrap_err();

    let res = app
        .execute_contract(
            Addr::unchecked(ALICE),
            cw20_addr,
            &Cw20ExecuteMsg::Send {
                contract: factory_addr.to_string(),
                amount: instantiate_payroll_msg.params.amount,
                msg: to_binary(&ReceiveMsg::InstantiatePayrollContract {
                    instantiate_msg: instantiate_payroll_msg,
                    code_id: cw_vesting_code_id,
                    label: "Payroll".to_string(),
                })
                .unwrap(),
            },
            &coins(amount.into(), NATIVE_DENOM),
        )
        .unwrap();

    // Get the payroll address from the instantiate event
    let instantiate_event = &res.events[4];
    assert_eq!(instantiate_event.ty, "instantiate");
    let cw_vesting_addr = instantiate_event.attributes[0].value.clone();

    // Check that admin of contract is owner specified in Instantiation Message
    let contract_info = app
        .wrap()
        .query_wasm_contract_info(cw_vesting_addr.clone())
        .unwrap();
    assert_eq!(contract_info.admin, Some(ALICE.to_string()));

    // Test query by instantiator
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr,
            &QueryMsg::ListVestingContractsByInstantiator {
                instantiator: ALICE.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Check that the vesting payment contract is active
    let vp: VestingPayment = app
        .wrap()
        .query_wasm_smart(cw_vesting_addr, &PayrollQueryMsg::Info {})
        .unwrap();
    assert_eq!(vp.status, VestingPaymentStatus::Active);
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

#[test]
fn test_instantiate_wrong_ownership_native() {
    let mut app = App::default();
    let code_id = app.store_code(factory_contract());
    let cw_vesting_code_id = app.store_code(cw_vesting_contract());

    // Alice is the owner. Contracts are only allowed if their owner
    // is alice or none and the sender is alice.
    let instantiate = InstantiateMsg {
        owner: Some(ALICE.to_string()),
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("ekez"),
            factory_addr.clone(),
            &ExecuteMsg::InstantiateNativePayrollContract {
                instantiate_msg: PayrollInstantiateMsg {
                    owner: Some(ALICE.to_string()),
                    params: UncheckedVestingParams {
                        recipient: BOB.to_string(),
                        amount: Uint128::new(10),
                        denom: UncheckedDenom::Native("tbucks".to_string()),
                        vesting_schedule: Curve::Constant {
                            y: Uint128::new(10),
                        },
                        title: None,
                        description: None,
                    },
                },
                code_id: cw_vesting_code_id,
                label: "vesting".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    // Can't instantiate if you are not the owner.
    assert_eq!(err, ContractError::Unauthorized {});

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ALICE),
            factory_addr,
            &ExecuteMsg::InstantiateNativePayrollContract {
                instantiate_msg: PayrollInstantiateMsg {
                    owner: Some("ekez".to_string()),
                    params: UncheckedVestingParams {
                        recipient: BOB.to_string(),
                        amount: Uint128::new(10),
                        denom: UncheckedDenom::Native("tbucks".to_string()),
                        vesting_schedule: Curve::Constant {
                            y: Uint128::new(10),
                        },
                        title: None,
                        description: None,
                    },
                },
                code_id: cw_vesting_code_id,
                label: "vesting".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    // Can't instantiate with an owner who is not the factory owner.
    assert_eq!(
        err,
        ContractError::OwnerMissmatch {
            actual: Some("ekez".to_string()),
            expected: Some(ALICE.to_string())
        }
    );
}

#[test]
fn test_instantiate_wrong_owner_cw20() {
    let mut app = App::default();
    let code_id = app.store_code(factory_contract());
    let cw20_code_id = app.store_code(cw20_contract());
    let cw_vesting_code_id = app.store_code(cw_vesting_contract());

    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(ALICE),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: ALICE.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    let instantiate = InstantiateMsg {
        owner: Some(ALICE.to_string()),
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Cw20(cw20_addr.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let instantiate_payroll_msg = PayrollInstantiateMsg {
        owner: Some(BOB.to_string()),
        params: UncheckedVestingParams {
            recipient: BOB.to_string(),
            amount: Uint128::new(1000000),
            denom: unchecked_denom,
            vesting_schedule,
            title: None,
            description: None,
        },
    };

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ALICE),
            cw20_addr,
            &Cw20ExecuteMsg::Send {
                contract: factory_addr.to_string(),
                amount: instantiate_payroll_msg.params.amount,
                msg: to_binary(&ReceiveMsg::InstantiatePayrollContract {
                    instantiate_msg: instantiate_payroll_msg,
                    code_id: cw_vesting_code_id,
                    label: "Payroll".to_string(),
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::OwnerMissmatch {
            actual: Some(BOB.to_string()),
            expected: Some(ALICE.to_string())
        }
    )
}
