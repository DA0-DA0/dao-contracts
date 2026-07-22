use std::collections::{BTreeMap, HashMap};

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, Uint128,
};

use crate::bindings::{JunoQuery, VotingPowerResponse};

pub const DAO_ADDR: &str = "dao-core";
pub const VOTER_A: &str = "voter-a";

#[derive(Default, Clone)]
pub struct SnapshotStore {
    per_addr: HashMap<String, BTreeMap<u64, String>>,
    total_at: BTreeMap<u64, String>,
}

impl SnapshotStore {
    pub fn set_power(&mut self, addr: &str, height: u64, power: u128) {
        self.set_raw_power(addr, height, &power.to_string());
    }

    pub fn set_raw_power(&mut self, addr: &str, height: u64, power: &str) {
        self.per_addr
            .entry(addr.to_string())
            .or_default()
            .insert(height, power.to_string());
    }

    pub fn set_total(&mut self, height: u64, power: u128) {
        self.total_at.insert(height, power.to_string());
    }

    pub fn set_raw_total(&mut self, height: u64, power: &str) {
        self.total_at.insert(height, power.to_string());
    }

    fn lookup_addr(&self, addr: &str, height: u64) -> String {
        self.per_addr
            .get(addr)
            .and_then(|snapshots| snapshots.range(..=height).next_back())
            .map(|(_, power)| power.clone())
            .unwrap_or_else(|| Uint128::zero().to_string())
    }

    fn lookup_total(&self, height: u64) -> String {
        self.total_at
            .range(..=height)
            .next_back()
            .map(|(_, power)| power.clone())
            .unwrap_or_else(|| Uint128::zero().to_string())
    }
}

pub struct JunoMockQuerier {
    base: MockQuerier,
    store: SnapshotStore,
}

impl JunoMockQuerier {
    pub fn new(store: SnapshotStore) -> Self {
        let base = MockQuerier::new(&[]);
        Self { base, store }
    }
}

impl Querier for JunoMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<JunoQuery> = match from_json(bin_request) {
            Ok(req) => req,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("parsing query: {e}"),
                    request: bin_request.into(),
                })
            }
        };
        match request {
            QueryRequest::Custom(custom) => self.handle_custom(custom, bin_request),
            other => self
                .base
                .raw_query(&cosmwasm_std::to_json_vec(&other).unwrap()),
        }
    }
}

impl JunoMockQuerier {
    fn handle_custom(&self, query: JunoQuery, raw: &[u8]) -> QuerierResult {
        let power = match query {
            JunoQuery::VotingPowerAt(params) => self
                .store
                .lookup_addr(&params.address, params.height as u64),
            JunoQuery::TotalVotingPowerAt(params) => self.store.lookup_total(params.height as u64),
            JunoQuery::VotingPowerOverRange(_) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: "VotingPowerOverRange not stubbed in tests".to_string(),
                    request: raw.into(),
                })
            }
        };
        SystemResult::Ok(ContractResult::Ok(
            to_json_binary(&VotingPowerResponse { power }).unwrap(),
        ))
    }
}

pub fn juno_deps_with(
    store: SnapshotStore,
) -> OwnedDeps<MockStorage, MockApi, JunoMockQuerier, JunoQuery> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: JunoMockQuerier::new(store),
        custom_query_type: std::marker::PhantomData,
    }
}
