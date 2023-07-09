#![cfg(test)]

use cosmwasm_std::{coins, Addr, Empty, Timestamp, Uint128};
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    registration::{Registration, RegistrationStatus},
    state::Config,
    ContractError,
};

const OWNER: &str = "owner";
const NOT_OWNER: &str = "not_owner";
const DAO1: &str = "dao1";
const DAO2: &str = "dao2";
const NAME1: &str = "name1";
const NAME2: &str = "name2";

const FEE_AMOUNT: u128 = 100;
const FEE_DENOM: &str = "denom";
const REGISTRATION_PERIOD_NANOS: u64 = 1_000_000_000;

const INITIAL_BALANCE: u128 = FEE_AMOUNT * 5;

fn setup_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn instantiate() -> (App, Addr) {
    let mut app = App::default();

    // Mint DAOs tokens to register.
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: DAO1.to_string(),
            amount: coins(INITIAL_BALANCE, FEE_DENOM),
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: DAO2.to_string(),
            amount: coins(INITIAL_BALANCE, FEE_DENOM),
        }
    }))
    .unwrap();

    // Instantiate contract.
    let code_id = app.store_code(setup_contract());
    let addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(OWNER),
            &InstantiateMsg {
                owner: OWNER.to_string(),
                fee_amount: Uint128::new(FEE_AMOUNT),
                fee_denom: UncheckedDenom::Native(FEE_DENOM.to_string()),
                registration_period: Timestamp::from_nanos(REGISTRATION_PERIOD_NANOS),
            },
            &[],
            "registry",
            None,
        )
        .unwrap();

    (app, addr)
}

#[test]
pub fn test_instantiate() {
    instantiate();
}

#[test]
pub fn test_updatable_owner() {
    let (mut app, addr) = instantiate();

    // Ensure owner is set.
    let res: cw_ownable::Ownership<String> = app
        .wrap()
        .query_wasm_smart(addr.clone(), &QueryMsg::Ownership {})
        .unwrap();
    assert_eq!(res.owner, Some(OWNER.to_string()));

    // Update owner.
    let new_owner = "new_owner";
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
            new_owner: new_owner.to_string(),
            expiry: None,
        }),
        &[],
    )
    .unwrap();

    // Accept ownership transfer.
    app.execute_contract(
        Addr::unchecked(new_owner),
        addr.clone(),
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::AcceptOwnership),
        &[],
    )
    .unwrap();

    // Ensure owner is updated to new owner.
    let res: cw_ownable::Ownership<String> = app
        .wrap()
        .query_wasm_smart(addr.clone(), &QueryMsg::Ownership {})
        .unwrap();
    assert_eq!(res.owner, Some(new_owner.to_string()));

    // Ensure old owner can no longer update.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            addr.clone(),
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: "new_new_owner".to_string(),
                expiry: None,
            }),
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Ownable(cw_ownable::OwnershipError::NotOwner)
    );

    // Disallow renouncing ownership.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(new_owner),
            addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::RenounceOwnership),
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::CannotRenounceOwnership);
}

#[test]
pub fn test_update_config() {
    let (mut app, addr) = instantiate();

    // Ensure config is set.
    let config: Config = app
        .wrap()
        .query_wasm_smart(addr.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        config,
        Config {
            fee_amount: Uint128::new(FEE_AMOUNT),
            fee_denom: CheckedDenom::Native(FEE_DENOM.to_string()),
            registration_period: Timestamp::from_nanos(REGISTRATION_PERIOD_NANOS),
        }
    );

    // Update config.
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::UpdateConfig {
            fee_amount: Some(Uint128::new(2 * FEE_AMOUNT)),
            fee_denom: Some(UncheckedDenom::Native("new_denom".to_string())),
            registration_period: Some(Timestamp::from_nanos(2 * REGISTRATION_PERIOD_NANOS)),
        },
        &[],
    )
    .unwrap();

    // Ensure config is updated.
    let new_config: Config = app
        .wrap()
        .query_wasm_smart(addr.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        new_config,
        Config {
            fee_amount: Uint128::new(2 * FEE_AMOUNT),
            fee_denom: CheckedDenom::Native("new_denom".to_string()),
            registration_period: Timestamp::from_nanos(2 * REGISTRATION_PERIOD_NANOS),
        }
    );

    // Ensure non-owner cannot update.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(NOT_OWNER),
            addr,
            &ExecuteMsg::UpdateConfig {
                fee_amount: Some(Uint128::new(2 * FEE_AMOUNT)),
                fee_denom: Some(UncheckedDenom::Native("new_denom".to_string())),
                registration_period: Some(Timestamp::from_nanos(2 * REGISTRATION_PERIOD_NANOS)),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Ownable(cw_ownable::OwnershipError::NotOwner)
    );
}

#[test]
pub fn test_register_approve() {
    let (mut app, addr) = instantiate();

    // Register with no funds.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            addr.clone(),
            &ExecuteMsg::Register {
                name: NAME1.to_string(),
                address: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::PaymentError(cw_utils::PaymentError::NoFunds {})
    );

    // Register with insufficient funds.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            addr.clone(),
            &ExecuteMsg::Register {
                name: NAME1.to_string(),
                address: None,
            },
            &coins(FEE_AMOUNT / 2, FEE_DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::WrongAmount);

    // Register with correct funds.
    app.execute_contract(
        Addr::unchecked(DAO1),
        addr.clone(),
        &ExecuteMsg::Register {
            name: NAME1.to_string(),
            address: None,
        },
        &coins(FEE_AMOUNT, FEE_DENOM),
    )
    .unwrap();

    // Ensure fee is stored in the contract.
    let balance = app
        .wrap()
        .query_balance(addr.clone(), FEE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(balance, Uint128::new(FEE_AMOUNT));

    // Ensure pending registration is created.
    let pending_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::PendingRegistration {
                address: DAO1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    let most_recent_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::MostRecentRegistration {
                address: DAO1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(pending_registration, most_recent_registration);
    assert!(pending_registration.is_pending());
    assert_eq!(
        pending_registration.address,
        Addr::unchecked(DAO1.to_string())
    );
    assert_eq!(pending_registration.name, NAME1.to_string());
    assert_eq!(pending_registration.expiration, Timestamp::from_nanos(0));

    // Ensure DAO not registered.
    let registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Registration {
                address: DAO1.to_string(),
            },
        )
        .unwrap();
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Resolve {
                name: NAME1.to_string(),
            },
        )
        .unwrap();
    assert!(registration.is_none());
    assert!(resolved_registration.is_none());

    // Approve registration by owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::Approve {
            address: DAO1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Ensure fee was transferred to the owner.
    let balance = app
        .wrap()
        .query_balance(addr.clone(), FEE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(balance, Uint128::zero());
    let balance = app.wrap().query_balance(OWNER, FEE_DENOM).unwrap().amount;
    assert_eq!(balance, Uint128::new(FEE_AMOUNT));

    // Ensure DAO registered.
    let pending_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::PendingRegistration {
                address: DAO1.to_string(),
            },
        )
        .unwrap();
    assert!(pending_registration.is_none());

    let registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Registration {
                address: DAO1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    let most_recent_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::MostRecentRegistration {
                address: DAO1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Resolve {
                name: NAME1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(registration, most_recent_registration);
    assert_eq!(registration, resolved_registration);
    assert_eq!(registration.address, Addr::unchecked(DAO1.to_string()));
    assert_eq!(registration.name, NAME1.to_string());
    assert_eq!(
        registration.expiration,
        app.block_info().time.plus_nanos(REGISTRATION_PERIOD_NANOS)
    );

    // Ensure DAO cannot register again.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO1),
            addr.clone(),
            &ExecuteMsg::Register {
                name: NAME2.to_string(),
                address: None,
            },
            &coins(FEE_AMOUNT, FEE_DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::AlreadyRegistered);

    // Ensure another DAO cannot register the same name.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(DAO2),
            addr,
            &ExecuteMsg::Register {
                name: NAME1.to_string(),
                address: None,
            },
            &coins(FEE_AMOUNT, FEE_DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NameAlreadyRegistered);
}

#[test]
pub fn test_register_reject() {
    let (mut app, addr) = instantiate();

    // Register.
    app.execute_contract(
        Addr::unchecked(DAO2),
        addr.clone(),
        &ExecuteMsg::Register {
            name: NAME2.to_string(),
            address: None,
        },
        &coins(FEE_AMOUNT, FEE_DENOM),
    )
    .unwrap();

    // Ensure fee is stored in the contract.
    let balance = app
        .wrap()
        .query_balance(addr.clone(), FEE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(balance, Uint128::new(FEE_AMOUNT));

    // Ensure pending registration is created.
    let pending_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::PendingRegistration {
                address: DAO2.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    let most_recent_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::MostRecentRegistration {
                address: DAO2.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(pending_registration, most_recent_registration);
    assert!(pending_registration.is_pending());
    assert_eq!(
        pending_registration.address,
        Addr::unchecked(DAO2.to_string())
    );
    assert_eq!(pending_registration.name, NAME2.to_string());
    assert_eq!(pending_registration.expiration, Timestamp::from_nanos(0));

    // Ensure DAO not registered.
    let registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Registration {
                address: DAO2.to_string(),
            },
        )
        .unwrap();
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Resolve {
                name: NAME2.to_string(),
            },
        )
        .unwrap();
    assert!(registration.is_none());
    assert!(resolved_registration.is_none());

    // Reject registration by owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::Reject {
            address: DAO2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Ensure fee was transferred back to the DAO.
    let balance = app
        .wrap()
        .query_balance(addr.clone(), FEE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(balance, Uint128::zero());
    let balance = app.wrap().query_balance(OWNER, FEE_DENOM).unwrap().amount;
    assert_eq!(balance, Uint128::zero());
    let balance = app.wrap().query_balance(DAO2, FEE_DENOM).unwrap().amount;
    assert_eq!(balance, Uint128::new(INITIAL_BALANCE));

    // Ensure DAO not registered.
    let pending_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::PendingRegistration {
                address: DAO2.to_string(),
            },
        )
        .unwrap();
    let registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Registration {
                address: DAO2.to_string(),
            },
        )
        .unwrap();
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Resolve {
                name: NAME2.to_string(),
            },
        )
        .unwrap();
    let most_recent_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::MostRecentRegistration {
                address: DAO2.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert!(pending_registration.is_none());
    assert!(registration.is_none());
    assert!(resolved_registration.is_none());
    assert_eq!(
        most_recent_registration.status,
        RegistrationStatus::Rejected
    );
    assert_eq!(
        most_recent_registration.address,
        Addr::unchecked(DAO2.to_string())
    );
    assert_eq!(most_recent_registration.name, NAME2.to_string());

    // Ensure DAO can register again.
    app.execute_contract(
        Addr::unchecked(DAO2),
        addr.clone(),
        &ExecuteMsg::Register {
            name: NAME2.to_string(),
            address: None,
        },
        &coins(FEE_AMOUNT, FEE_DENOM),
    )
    .unwrap();

    // Ensure another DAO can register the same name.
    app.execute_contract(
        Addr::unchecked(DAO1),
        addr,
        &ExecuteMsg::Register {
            name: NAME2.to_string(),
            address: None,
        },
        &coins(FEE_AMOUNT, FEE_DENOM),
    )
    .unwrap();
}

#[test]
pub fn test_owner_privileges() {
    let (mut app, addr) = instantiate();

    // Register by owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::Register {
            name: NAME1.to_string(),
            address: Some(DAO1.to_string()),
        },
        &[],
    )
    .unwrap();

    // Ensure DAO registered.
    let pending_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::PendingRegistration {
                address: DAO1.to_string(),
            },
        )
        .unwrap();
    assert!(pending_registration.is_none());

    let registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Registration {
                address: DAO1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    let most_recent_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::MostRecentRegistration {
                address: DAO1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Resolve {
                name: NAME1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(registration, most_recent_registration);
    assert_eq!(registration, resolved_registration);
    assert_eq!(registration.address, Addr::unchecked(DAO1.to_string()));
    assert_eq!(registration.name, NAME1.to_string());
    assert_eq!(
        registration.expiration,
        app.block_info().time.plus_nanos(REGISTRATION_PERIOD_NANOS)
    );

    // Update expiration by owner.
    let new_expiration = registration.expiration.plus_nanos(100);
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::UpdateExpiration {
            name: NAME1.to_string(),
            expiration: new_expiration,
        },
        &[],
    )
    .unwrap();

    // Ensure DAO registration updated.
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Resolve {
                name: NAME1.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(resolved_registration.expiration, new_expiration);

    // Revoke registration by owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        addr.clone(),
        &ExecuteMsg::Revoke {
            name: NAME1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Ensure DAO not registered.
    let registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr.clone(),
            &QueryMsg::Registration {
                address: DAO1.to_string(),
            },
        )
        .unwrap();
    let resolved_registration = app
        .wrap()
        .query_wasm_smart::<Option<Registration>>(
            addr,
            &QueryMsg::Resolve {
                name: NAME1.to_string(),
            },
        )
        .unwrap();
    assert!(registration.is_none());
    assert!(resolved_registration.is_none());
}
