use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, CosmosMsg, WasmMsg};
use dao_hooks::stake::{StakeChangedExecuteMsg, StakeChangedHookMsg};

use crate::contract::{execute, instantiate, sudo};
use crate::msg::{
    DelegationEvent, ExecuteMsg, InstantiateMsg, SudoMsg, ValidatorEvent, ValidatorSlashEvent,
};

use super::support::{juno_deps_with, SnapshotStore, DAO_ADDR, VOTER_A};

fn boot(
    store: SnapshotStore,
) -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::testing::MockStorage,
    cosmwasm_std::testing::MockApi,
    super::support::JunoMockQuerier,
    crate::bindings::JunoQuery,
> {
    let mut deps = juno_deps_with(store);
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        InstantiateMsg {
            auto_register_staking_hooks: Some(false),
        },
    )
    .unwrap();
    deps
}

fn subscribe(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::testing::MockStorage,
        cosmwasm_std::testing::MockApi,
        super::support::JunoMockQuerier,
        crate::bindings::JunoQuery,
    >,
    subscriber: &str,
) {
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        ExecuteMsg::AddHook {
            addr: subscriber.to_string(),
        },
    )
    .unwrap();
}

fn extract_hook_msg(msg: &CosmosMsg) -> StakeChangedHookMsg {
    match msg {
        CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) => {
            let env: StakeChangedExecuteMsg = from_json(msg).unwrap();
            match env {
                StakeChangedExecuteMsg::StakeChangeHook(h) => h,
            }
        }
        _ => panic!("expected wasm execute"),
    }
}

#[test]
fn after_delegation_modified_fires_stake_delta() {
    let mut store = SnapshotStore::default();
    // Pre-event: at the height before sudo, voter had 100. Post-event:
    // chain wrote 175 at current height. Delta = +75.
    let env = mock_env();
    let current_height = env.block.height;
    store.set_power(VOTER_A, current_height - 1, 100);
    store.set_power(VOTER_A, current_height, 175);
    let mut deps = boot(store);
    subscribe(&mut deps, "subscriber");

    let resp = sudo(
        deps.as_mut(),
        env,
        SudoMsg::AfterDelegationModified {
            after_delegation_modified: DelegationEvent {
                delegator_address: VOTER_A.to_string(),
                validator_address: "junovaloper1...".to_string(),
                shares: "75000000".to_string(),
            },
        },
    )
    .unwrap();

    assert_eq!(resp.messages.len(), 1);
    let hook = extract_hook_msg(&resp.messages[0].msg);
    match hook {
        StakeChangedHookMsg::Stake { addr, amount } => {
            assert_eq!(addr.as_str(), VOTER_A);
            assert_eq!(amount.u128(), 75);
        }
        other => panic!("expected Stake, got {other:?}"),
    }
}

#[test]
fn after_delegation_modified_fires_unstake_when_power_drops() {
    let mut store = SnapshotStore::default();
    let env = mock_env();
    let h = env.block.height;
    // Voter previously had 200, partial undelegation drops them to 80.
    store.set_power(VOTER_A, h - 1, 200);
    store.set_power(VOTER_A, h, 80);
    let mut deps = boot(store);
    subscribe(&mut deps, "subscriber");

    let resp = sudo(
        deps.as_mut(),
        env,
        SudoMsg::AfterDelegationModified {
            after_delegation_modified: DelegationEvent {
                delegator_address: VOTER_A.to_string(),
                validator_address: "junovaloper1...".to_string(),
                shares: "80000000".to_string(),
            },
        },
    )
    .unwrap();

    assert_eq!(resp.messages.len(), 1);
    let hook = extract_hook_msg(&resp.messages[0].msg);
    match hook {
        StakeChangedHookMsg::Unstake { addr, amount } => {
            assert_eq!(addr.as_str(), VOTER_A);
            assert_eq!(amount.u128(), 120);
        }
        other => panic!("expected Unstake, got {other:?}"),
    }
}

#[test]
fn before_delegation_removed_emits_full_unstake() {
    let mut store = SnapshotStore::default();
    let env = mock_env();
    let h = env.block.height;
    // Voter has 90 just before the removal commits.
    store.set_power(VOTER_A, h - 1, 90);
    store.set_power(VOTER_A, h, 90);
    let mut deps = boot(store);
    subscribe(&mut deps, "subscriber");

    let resp = sudo(
        deps.as_mut(),
        env,
        SudoMsg::BeforeDelegationRemoved {
            before_delegation_removed: DelegationEvent {
                delegator_address: VOTER_A.to_string(),
                validator_address: "junovaloper1...".to_string(),
                shares: "0".to_string(),
            },
        },
    )
    .unwrap();

    let hook = extract_hook_msg(&resp.messages[0].msg);
    match hook {
        StakeChangedHookMsg::Unstake { amount, .. } => assert_eq!(amount.u128(), 90),
        other => panic!("expected Unstake, got {other:?}"),
    }
}

#[test]
fn unchanged_power_emits_no_hooks() {
    let mut store = SnapshotStore::default();
    let env = mock_env();
    let h = env.block.height;
    store.set_power(VOTER_A, h - 1, 100);
    store.set_power(VOTER_A, h, 100);
    let mut deps = boot(store);
    subscribe(&mut deps, "subscriber");

    let resp = sudo(
        deps.as_mut(),
        env,
        SudoMsg::AfterDelegationModified {
            after_delegation_modified: DelegationEvent {
                delegator_address: VOTER_A.to_string(),
                validator_address: "junovaloper1...".to_string(),
                shares: "100000000".to_string(),
            },
        },
    )
    .unwrap();
    assert!(resp.messages.is_empty());
}

#[test]
fn fan_out_targets_every_subscriber() {
    let mut store = SnapshotStore::default();
    let env = mock_env();
    let h = env.block.height;
    store.set_power(VOTER_A, h - 1, 0);
    store.set_power(VOTER_A, h, 42);
    let mut deps = boot(store);
    subscribe(&mut deps, "gauge");
    subscribe(&mut deps, "rewards");

    let resp = sudo(
        deps.as_mut(),
        env,
        SudoMsg::AfterDelegationModified {
            after_delegation_modified: DelegationEvent {
                delegator_address: VOTER_A.to_string(),
                validator_address: "junovaloper1...".to_string(),
                shares: "42000000".to_string(),
            },
        },
    )
    .unwrap();
    assert_eq!(resp.messages.len(), 2);
    let targets: Vec<String> = resp
        .messages
        .iter()
        .map(|m| match &m.msg {
            CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => contract_addr.clone(),
            _ => panic!(),
        })
        .collect();
    assert!(targets.contains(&"gauge".to_string()));
    assert!(targets.contains(&"rewards".to_string()));
}

#[test]
fn validator_lifecycle_and_slash_events_are_swallowed_silently() {
    let mut deps = boot(SnapshotStore::default());
    subscribe(&mut deps, "subscriber");

    let validator = ValidatorEvent {
        moniker: "v".to_string(),
        validator_address: "junovaloper1...".to_string(),
        commission: "0.05".to_string(),
        validator_tokens: "0".to_string(),
        bonded_tokens: "0".to_string(),
        bond_status: "BOND_STATUS_BONDED".to_string(),
    };

    for msg in [
        SudoMsg::AfterValidatorCreated {
            after_validator_created: validator.clone(),
        },
        SudoMsg::AfterValidatorRemoved {
            after_validator_removed: validator.clone(),
        },
        SudoMsg::AfterValidatorBonded {
            after_validator_bonded: validator.clone(),
        },
        SudoMsg::AfterValidatorBeginUnbonding {
            after_validator_begin_unbonding: validator.clone(),
        },
        SudoMsg::BeforeValidatorModified {
            before_validator_modified: validator.clone(),
        },
        SudoMsg::AfterValidatorModified {
            after_validator_modified: validator.clone(),
        },
        SudoMsg::BeforeValidatorSlashed {
            before_validator_slashed: ValidatorSlashEvent {
                moniker: "v".to_string(),
                validator_address: "junovaloper1...".to_string(),
                slashed_amount: "0.01".to_string(),
            },
        },
    ] {
        let resp = sudo(deps.as_mut(), mock_env(), msg).unwrap();
        assert!(resp.messages.is_empty());
    }
}
