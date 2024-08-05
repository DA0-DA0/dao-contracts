use cw_orch::{interface, prelude::*};

#[interface(
    gauge_orchestrator::msg::InstantiateMsg,
    gauge_orchestrator::msg::ExecuteMsg,
    gauge_orchestrator::msg::QueryMsg,
    Empty
)]
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
            ContractWrapper::new_with_empty(
                gauge_orchestrator::contract::execute,
                gauge_orchestrator::contract::instantiate,
                gauge_orchestrator::contract::query,
            )
            .with_migrate(gauge_orchestrator::contract::migrate),
        )
    }
}

#[interface(
    gauge_adapter::msg::InstantiateMsg,
    gauge_adapter::msg::ExecuteMsg,
    gauge_adapter::msg::AdapterQueryMsg,
    Empty
)]
pub struct DaoGaugeAdapter;

impl<Chain> Uploadable for DaoGaugeAdapter<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("gauge_adapter")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            gauge_adapter::contract::execute,
            gauge_adapter::contract::instantiate,
            gauge_adapter::contract::query,
        ))
    }
}
