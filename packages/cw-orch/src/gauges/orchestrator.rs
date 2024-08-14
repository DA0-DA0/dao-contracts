use cw_orch::{interface, prelude::*};
use gauge_orchestrator::contract::{execute, instantiate, query};
use gauge_orchestrator::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct DaoGaugeOrchestrator;

impl<Chain> Uploadable for DaoGaugeOrchestrator<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("gauge_orchestrator")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(execute, instantiate, query)
                .with_migrate(gauge_orchestrator::contract::migrate),
        )
    }
}
