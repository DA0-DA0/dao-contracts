use std::collections::HashMap;

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, Uint128,
};

use crate::bindings::{JunoQuery, VotingPowerResponse};

pub const DAO_ADDR: &str = "dao-core";
pub const VOTER_A: &str = "voter-a";
pub const VOTER_B: &str = "voter-b";

/// Canned snapshot store keyed on `(address, height)`. A height of `None`
/// in the lookup acts as a fallback for "any height not explicitly set",
/// matching the at-or-before semantics of the real chain module.
#[derive(Default, Clone)]
pub struct SnapshotStore {
    per_addr: HashMap<(String, u64), Uint128>,
    addr_default: HashMap<String, Uint128>,
    total_at: HashMap<u64, Uint128>,
    total_default: Uint128,
}

impl SnapshotStore {
    pub fn set_power(&mut self, addr: &str, height: u64, power: u128) {
        self.per_addr
            .insert((addr.to_string(), height), Uint128::new(power));
    }

    pub fn set_default_power(&mut self, addr: &str, power: u128) {
        self.addr_default
            .insert(addr.to_string(), Uint128::new(power));
    }

    pub fn set_total(&mut self, height: u64, power: u128) {
        self.total_at.insert(height, Uint128::new(power));
    }

    pub fn set_default_total(&mut self, power: u128) {
        self.total_default = Uint128::new(power);
    }

    fn lookup_addr(&self, addr: &str, height: u64) -> Uint128 {
        if let Some(p) = self.per_addr.get(&(addr.to_string(), height)) {
            return *p;
        }
        self.addr_default
            .get(addr)
            .copied()
            .unwrap_or(Uint128::zero())
    }

    fn lookup_total(&self, height: u64) -> Uint128 {
        self.total_at
            .get(&height)
            .copied()
            .unwrap_or(self.total_default)
    }
}

pub struct JunoMockQuerier {
    base: MockQuerier,
    store: SnapshotStore,
}

impl JunoMockQuerier {
    pub fn new(store: SnapshotStore) -> Self {
        Self {
            base: MockQuerier::new(&[]),
            store,
        }
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
    fn handle_custom(&self, q: JunoQuery, raw: &[u8]) -> QuerierResult {
        let resp = match q {
            JunoQuery::VotingPowerAt(p) => VotingPowerResponse {
                power: self
                    .store
                    .lookup_addr(&p.address, p.height as u64)
                    .to_string(),
            },
            JunoQuery::TotalVotingPowerAt(p) => VotingPowerResponse {
                power: self.store.lookup_total(p.height as u64).to_string(),
            },
            JunoQuery::VotingPowerOverRange(_) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: "VotingPowerOverRange not stubbed in tests".to_string(),
                    request: raw.into(),
                })
            }
        };
        SystemResult::Ok(ContractResult::Ok(to_json_binary(&resp).unwrap()))
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
