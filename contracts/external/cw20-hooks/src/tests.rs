use cosmwasm_std::{Addr, Empty, Uint128};
use cw20::{Cw20Coin, MinterResponse};
use cw_controllers::HooksResponse;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_ownable::Ownership;

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

fn cw20_hooks_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply)
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}

const OWNER: &str = "owner";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const NOONE: &str = "noone";

fn setup_contract(app: &mut App) -> Addr {
    let cw20_hooks_code_id = app.store_code(cw20_hooks_contract());

    let initial_balances = vec![
        Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::new(100_000_000),
        },
        Cw20Coin {
            address: ADDR2.to_string(),
            amount: Uint128::new(100_000_000),
        },
        Cw20Coin {
            address: ADDR3.to_string(),
            amount: Uint128::new(100_000_000),
        },
    ];

    // Instantiate cw20-hooks contract.
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        name: "name".to_string(),
        symbol: "symbol".to_string(),
        decimals: 6,
        initial_balances,
        mint: Some(MinterResponse {
            minter: OWNER.to_string(),
            cap: Some(Uint128::new(500_000_000)),
        }),
        marketing: None,
    };

    app.instantiate_contract(
        cw20_hooks_code_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        "cw20-hooks",
        None,
    )
    .unwrap()
}

#[test]
pub fn test_instantiate() {
    let mut app = App::default();
    setup_contract(&mut app);
}

#[test]
pub fn test_add_remove_hook() {
    let mut app = App::default();
    let cw20_hooks_addr = setup_contract(&mut app);

    // Ensure no hooks have been registered.
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Hooks {})
        .unwrap();
    assert_eq!(hooks, HooksResponse { hooks: vec![] });

    // Fail to add if not owner.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(NOONE),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::AddHook {
                addr: ADDR2.to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Add successfully if owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::AddHook {
            addr: ADDR2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Ensure 1 hook registered.
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Hooks {})
        .unwrap();
    assert_eq!(
        hooks,
        HooksResponse {
            hooks: vec![ADDR2.to_string()]
        }
    );

    // Fail to remove if not owner.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(NOONE),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::RemoveHook {
                addr: ADDR2.to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Remove successfully if owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::RemoveHook {
            addr: ADDR2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Ensure no hooks registered.
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Hooks {})
        .unwrap();
    assert_eq!(hooks, HooksResponse { hooks: vec![] });

    // Remove owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::RenounceOwnership {}),
        &[],
    )
    .unwrap();

    // Owner can no longer add nor remove hooks.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::AddHook {
                addr: ADDR2.to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::RemoveHook {
                addr: ADDR2.to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
pub fn test_ownership_transfer() {
    let mut app = App::default();
    let cw20_hooks_addr = setup_contract(&mut app);

    // Ensure owner is set.
    let ownership: Ownership<Addr> = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Ownership {})
        .unwrap();
    assert_eq!(
        ownership,
        Ownership {
            owner: Some(Addr::unchecked(OWNER)),
            pending_owner: None,
            pending_expiry: None,
        }
    );

    // Fail to transfer owner if not owner.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(NOONE),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: NOONE.to_string(),
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

    // Initiate transfer if owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
            new_owner: ADDR3.to_string(),
            expiry: None,
        }),
        &[],
    )
    .unwrap();

    // Accept transfer from new owner.
    app.execute_contract(
        Addr::unchecked(ADDR3),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::AcceptOwnership {}),
        &[],
    )
    .unwrap();

    // Ensure owner was transferred.
    let ownership: Ownership<Addr> = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Ownership {})
        .unwrap();
    assert_eq!(
        ownership,
        Ownership {
            owner: Some(Addr::unchecked(ADDR3)),
            pending_owner: None,
            pending_expiry: None,
        }
    );
}

#[test]
fn owner_can_update_minter_but_not_cap() {
    let mut app = App::default();
    let cw20_hooks_addr = setup_contract(&mut app);

    // Ensure minter set.
    let minter: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Minter {})
        .unwrap();
    assert_eq!(
        minter,
        Some(MinterResponse {
            minter: OWNER.to_string(),
            cap: Some(Uint128::new(500_000_000))
        })
    );

    // Change minter.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMinter {
            new_minter: Some(ADDR2.to_string()),
        },
        &[],
    )
    .unwrap();

    // Ensure minter changed with same cap as before.
    let minter: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Minter {})
        .unwrap();
    assert_eq!(
        minter,
        Some(MinterResponse {
            minter: ADDR2.to_string(),
            cap: Some(Uint128::new(500_000_000))
        })
    );

    // Remove minter.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMinter { new_minter: None },
        &[],
    )
    .unwrap();

    // Ensure minter cleared.
    let minter: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Minter {})
        .unwrap();
    assert_eq!(minter, None);

    // Set minter again.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMinter {
            new_minter: Some(ADDR3.to_string()),
        },
        &[],
    )
    .unwrap();

    // Ensure minter set again with same cap as before.
    let minter: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(cw20_hooks_addr.clone(), &QueryMsg::Minter {})
        .unwrap();
    assert_eq!(
        minter,
        Some(MinterResponse {
            minter: ADDR3.to_string(),
            cap: Some(Uint128::new(500_000_000))
        })
    );
}

#[test]
fn owner_can_update_marketing_info() {
    let mut app = App::default();
    let cw20_hooks_addr = setup_contract(&mut app);

    // Set marketing info.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMarketing {
            project: Some("project".to_string()),
            description: Some("description".to_string()),
            marketing: Some(ADDR2.to_string()),
        },
        &[],
    )
    .unwrap();

    // Ensure marketer can update marketing info.
    app.execute_contract(
        Addr::unchecked(ADDR2),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMarketing {
            project: Some("new_project".to_string()),
            description: None,
            marketing: None,
        },
        &[],
    )
    .unwrap();

    // Ensure owner can update marketing info and clear marketer.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMarketing {
            project: Some("new_new_project".to_string()),
            description: Some("new_new_description".to_string()),
            marketing: Some("".to_string()),
        },
        &[],
    )
    .unwrap();

    // Ensure marketer can no longer update marketing info.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ADDR2),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::UpdateMarketing {
                project: Some("project".to_string()),
                description: None,
                marketing: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Ensure owner can update marketing info even if marketer was unset.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMarketing {
            project: Some("old_project".to_string()),
            description: None,
            marketing: Some(ADDR3.to_string()),
        },
        &[],
    )
    .unwrap();

    // Remove owner.
    app.execute_contract(
        Addr::unchecked(OWNER),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::RenounceOwnership {}),
        &[],
    )
    .unwrap();

    // Owner can no longer update marketing info.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            cw20_hooks_addr.clone(),
            &ExecuteMsg::UpdateMarketing {
                project: Some("another_project".to_string()),
                description: None,
                marketing: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Ensure marketer can still update marketing info.
    app.execute_contract(
        Addr::unchecked(ADDR3),
        cw20_hooks_addr.clone(),
        &ExecuteMsg::UpdateMarketing {
            project: Some("my_project".to_string()),
            description: None,
            marketing: None,
        },
        &[],
    )
    .unwrap();
}
