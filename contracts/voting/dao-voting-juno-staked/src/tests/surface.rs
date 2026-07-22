use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, to_json_vec, Addr, Uint128};
use cw2::{get_contract_version, set_contract_version};
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::bindings::{JunoQuery, TotalVotingPowerAt, VotingPowerAt};
use crate::contract::{instantiate, migrate, query, CONTRACT_NAME, CONTRACT_VERSION};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use super::support::{juno_deps_with, SnapshotStore, DAO_ADDR, VOTER_A};

fn instantiate_module(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::testing::MockStorage,
        cosmwasm_std::testing::MockApi,
        super::support::JunoMockQuerier,
        crate::bindings::JunoQuery,
    >,
) {
    let response = instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(DAO_ADDR, &[]),
        InstantiateMsg {},
    )
    .unwrap();
    assert_eq!(response.attributes[0].value, "instantiate");
}

#[test]
fn custom_query_json_matches_juno_v30_wire_format() {
    assert_eq!(
        to_json_vec(&JunoQuery::VotingPowerAt(VotingPowerAt {
            address: VOTER_A.to_string(),
            height: 41,
        }))
        .unwrap(),
        br#"{"voting_power_at":{"address":"voter-a","height":41}}"#
    );
    assert_eq!(
        to_json_vec(&JunoQuery::TotalVotingPowerAt(TotalVotingPowerAt {
            height: 41,
        }))
        .unwrap(),
        br#"{"total_voting_power_at":{"height":41}}"#
    );
}

#[test]
fn instantiate_sets_version_and_queries_instantiating_dao_and_info() {
    let mut deps = juno_deps_with(SnapshotStore::default());
    instantiate_module(&mut deps);

    assert_eq!(
        get_contract_version(&deps.storage).unwrap(),
        cw2::ContractVersion {
            contract: CONTRACT_NAME.to_string(),
            version: CONTRACT_VERSION.to_string(),
        }
    );

    let dao: Addr = from_json(query(deps.as_ref(), mock_env(), QueryMsg::Dao {}).unwrap()).unwrap();
    assert_eq!(dao, Addr::unchecked(DAO_ADDR));

    let info: InfoResponse =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Info {}).unwrap()).unwrap();
    assert_eq!(info.info.contract, CONTRACT_NAME);
    assert_eq!(info.info.version, CONTRACT_VERSION);
}

#[test]
fn all_power_queries_use_beginning_of_block_snapshots() {
    let mut store = SnapshotStore::default();
    store.set_power(VOTER_A, 99, 250_000_000);
    store.set_power(VOTER_A, 249, 300_000_000);
    store.set_total(99, 1_000_000_000);
    store.set_total(249, 1_200_000_000);
    let mut deps = juno_deps_with(store);
    instantiate_module(&mut deps);

    let mut env = mock_env();
    env.block.height = 250;

    let historical: VotingPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::VotingPowerAtHeight {
                address: VOTER_A.to_string(),
                height: Some(100),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(historical.power, Uint128::new(250_000_000));
    assert_eq!(historical.height, 100);

    let current: VotingPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::VotingPowerAtHeight {
                address: VOTER_A.to_string(),
                height: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(current.power, Uint128::new(300_000_000));
    assert_eq!(current.height, 250);

    let historical_total: TotalPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::TotalPowerAtHeight { height: Some(100) },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(historical_total.power, Uint128::new(1_000_000_000));
    assert_eq!(historical_total.height, 100);

    let current_total: TotalPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::TotalPowerAtHeight { height: None },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(current_total.power, Uint128::new(1_200_000_000));
    assert_eq!(current_total.height, 250);
}

#[test]
fn sparse_snapshots_carry_forward_at_or_before_the_requested_height() {
    let mut store = SnapshotStore::default();
    store.set_power(VOTER_A, 99, 250_000_000);
    store.set_total(99, 1_000_000_000);
    let deps = juno_deps_with(store);

    let voter: VotingPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::VotingPowerAtHeight {
                address: VOTER_A.to_string(),
                height: Some(150),
            },
        )
        .unwrap(),
    )
    .unwrap();
    let total: TotalPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::TotalPowerAtHeight { height: Some(150) },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(voter.power, Uint128::new(250_000_000));
    assert_eq!(total.power, Uint128::new(1_000_000_000));
}

#[test]
fn genesis_boundary_does_not_underflow() {
    let mut store = SnapshotStore::default();
    store.set_power(VOTER_A, 0, 7);
    store.set_total(0, 11);
    let deps = juno_deps_with(store);

    let voter: VotingPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::VotingPowerAtHeight {
                address: VOTER_A.to_string(),
                height: Some(0),
            },
        )
        .unwrap(),
    )
    .unwrap();
    let total: TotalPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::TotalPowerAtHeight { height: Some(0) },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(voter.power, Uint128::new(7));
    assert_eq!(voter.height, 0);
    assert_eq!(total.power, Uint128::new(11));
    assert_eq!(total.height, 0);
}

#[test]
fn malformed_overflow_and_out_of_range_chain_values_are_rejected() {
    let mut malformed = SnapshotStore::default();
    malformed.set_raw_power(VOTER_A, 6, "not-a-number");
    let deps = juno_deps_with(malformed);
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::VotingPowerAtHeight {
            address: VOTER_A.to_string(),
            height: Some(7),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("unparseable power"));

    let mut overflow = SnapshotStore::default();
    overflow.set_raw_total(6, "340282366920938463463374607431768211456");
    let deps = juno_deps_with(overflow);
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::TotalPowerAtHeight { height: Some(7) },
    )
    .unwrap_err();
    assert!(err.to_string().contains("unparseable power"));

    let mut boundary = SnapshotStore::default();
    boundary.set_total(i64::MAX as u64, 42);
    let deps = juno_deps_with(boundary);
    let response: TotalPowerAtHeightResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::TotalPowerAtHeight {
                height: Some(i64::MAX as u64 + 1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(response.power, Uint128::new(42));

    let deps = juno_deps_with(SnapshotStore::default());
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::TotalPowerAtHeight {
            height: Some(i64::MAX as u64 + 2),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("exceeds i64::MAX"));
}

#[test]
fn execute_schema_has_no_supported_messages() {
    assert!(from_json::<ExecuteMsg>(br#"{}"#).is_err());
    assert!(from_json::<ExecuteMsg>(br#"{"add_hook":{"addr":"hook"}}"#).is_err());
}

#[test]
fn migration_requires_an_older_matching_contract_and_updates_cw2() {
    let mut deps = juno_deps_with(SnapshotStore::default());
    set_contract_version(&mut deps.storage, CONTRACT_NAME, "2.7.0").unwrap();
    let response = migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    assert_eq!(response.attributes[0].value, "migrate");
    assert_eq!(
        get_contract_version(&deps.storage).unwrap().version,
        CONTRACT_VERSION
    );

    let mut mismatch = juno_deps_with(SnapshotStore::default());
    set_contract_version(&mut mismatch.storage, "wrong-contract", "2.7.0").unwrap();
    assert!(migrate(mismatch.as_mut(), mock_env(), MigrateMsg {}).is_err());

    let mut same = juno_deps_with(SnapshotStore::default());
    set_contract_version(&mut same.storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();
    assert!(migrate(same.as_mut(), mock_env(), MigrateMsg {}).is_err());

    let mut newer = juno_deps_with(SnapshotStore::default());
    set_contract_version(&mut newer.storage, CONTRACT_NAME, "99.0.0").unwrap();
    assert!(migrate(newer.as_mut(), mock_env(), MigrateMsg {}).is_err());
}
