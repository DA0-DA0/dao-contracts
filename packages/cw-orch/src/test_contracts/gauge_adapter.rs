use cw_orch::{interface, prelude::*};

use dao_gauge_adapter::contract::{execute, instantiate, query, ExecuteMsg, InstantiateMsg};
use gauge_adapter::msg::AdapterQueryMsg;

#[interface(InstantiateMsg, ExecuteMsg, AdapterQueryMsg, Empty)]
pub struct DaoGaugeAdapterGeneric;

impl<Chain> Uploadable for DaoGaugeAdapterGeneric<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("dao_proposal_hook_counter")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
