use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, Addr, Uint128};
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetHooksResponse, InstantiateMsg, QueryMsg};

use super::support::{juno_deps_with, SnapshotStore, DAO_ADDR, VOTER_A, VOTER_B};

fn instantiate_module(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::testing::MockStorage,
        cosmwasm_std::testing::MockApi,
        super::support::JunoMockQuerier,
        crate::bindings::JunoQuery,
    >,
) {
    let info = mock_info(DAO_ADDR, &[]);
    instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            auto_register_staking_hooks: Some(false),
        },
    )
    .unwrap();
}

#[test]
fn instantiate_stores_dao_address() {
    let mut store = SnapshotStore::default();
    store.set_default_total(0);
    let mut deps = juno_deps_with(store);
    instantiate_module(&mut deps);

    let dao_bin = query(deps.as_ref(), mock_env(), QueryMsg::Dao {}).unwrap();
    let dao: Addr = from_json(dao_bin).unwrap();
    assert_eq!(dao, Addr::unchecked(DAO_ADDR));

    let info_bin = query(deps.as_ref(), mock_env(), QueryMsg::Info {}).unwrap();
    let info: InfoResponse = from_json(info_bin).unwrap();
    assert_eq!(info.info.contract, "crates.io:dao-voting-juno-staked");
}

#[test]
fn voting_power_proxies_to_chain_snapshot_with_at_or_before_semantics() {
    let mut store = SnapshotStore::default();
    // The chain wrote the voter's power at height 100; later heights
    // without an explicit snapshot fall through to whatever the
    // default for that voter is. Mirrors the chain's at-or-before
    // iterator returning the latest entry <= requested height.
    store.set_power(VOTER_A, 100, 250_000_000);
    store.set_default_power(VOTER_A, 250_000_000);
    store.set_total(100, 1_000_000_000);
    store.set_default_total(1_000_000_000);
    let mut deps = juno_deps_with(store);
    instantiate_module(&mut deps);

    let mut env = mock_env();
    env.block.height = 250;

    let bin = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::VotingPowerAtHeight {
            address: VOTER_A.to_string(),
            height: Some(100),
        },
    )
    .unwrap();
    let resp: VotingPowerAtHeightResponse = from_json(bin).unwrap();
    assert_eq!(resp.power, Uint128::new(250_000_000));
    assert_eq!(resp.height, 100);

    // No height → current env.block.height.
    let bin = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::VotingPowerAtHeight {
            address: VOTER_A.to_string(),
            height: None,
        },
    )
    .unwrap();
    let resp: VotingPowerAtHeightResponse = from_json(bin).unwrap();
    assert_eq!(resp.power, Uint128::new(250_000_000));
    assert_eq!(resp.height, 250);

    let bin = query(
        deps.as_ref(),
        env,
        QueryMsg::TotalPowerAtHeight { height: Some(100) },
    )
    .unwrap();
    let resp: TotalPowerAtHeightResponse = from_json(bin).unwrap();
    assert_eq!(resp.power, Uint128::new(1_000_000_000));
}

#[test]
fn add_and_remove_hook_only_dao() {
    let mut deps = juno_deps_with(SnapshotStore::default());
    instantiate_module(&mut deps);

    // Non-DAO sender is rejected.
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("rando", &[]),
        ExecuteMsg::AddHook {
            addr: VOTER_A.to_string(),
        },
    );
    assert!(matches!(res, Err(ContractError::Unauthorized {})));

    // DAO adds two subscribers.
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        ExecuteMsg::AddHook {
            addr: VOTER_A.to_string(),
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        ExecuteMsg::AddHook {
            addr: VOTER_B.to_string(),
        },
    )
    .unwrap();

    let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetHooks {}).unwrap();
    let resp: GetHooksResponse = from_json(bin).unwrap();
    assert_eq!(resp.hooks.len(), 2);
    assert!(resp.hooks.contains(&VOTER_A.to_string()));
    assert!(resp.hooks.contains(&VOTER_B.to_string()));

    // Duplicate add fails.
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        ExecuteMsg::AddHook {
            addr: VOTER_A.to_string(),
        },
    );
    assert!(matches!(res, Err(ContractError::HookError(_))));

    // Remove works.
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        ExecuteMsg::RemoveHook {
            addr: VOTER_A.to_string(),
        },
    )
    .unwrap();
    let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetHooks {}).unwrap();
    let resp: GetHooksResponse = from_json(bin).unwrap();
    assert_eq!(resp.hooks, vec![VOTER_B.to_string()]);
}
