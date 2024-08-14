use cw_orch::{interface, prelude::*};
use gauge_adapter::contract::{execute, instantiate, query};
use gauge_adapter::msg::{AdapterQueryMsg, ExecuteMsg, InstantiateMsg};

#[interface(InstantiateMsg, ExecuteMsg, AdapterQueryMsg, Empty)]
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
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
