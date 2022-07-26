use anyhow::Result;
use cosm_orc::orchestrator::cosm_orc::{CosmOrc, WasmMsg};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use std::{fs, sync::Mutex};

static COSM_ORC: OnceCell<Mutex<Chain>> = OnceCell::new();

// Chain gives all the tests access to a global CosmOrc singleton
#[derive(Debug)]
pub struct Chain {
    contract_dir: String,
    gas_report_out: String,
    cosm_orc: CosmOrc,
}
impl Chain {
    pub fn init(cosm_orc: CosmOrc, contract_dir: String, gas_report_out: String) {
        COSM_ORC
            .set(Mutex::new(Chain {
                contract_dir,
                gas_report_out,
                cosm_orc,
            }))
            .expect("error initializing cosm-orc");
    }

    fn get() -> &'static Mutex<Chain> {
        COSM_ORC.get().expect("cosm-orc is not initialized")
    }

    // returns the deployed code_id for the given contract name
    pub fn contract_code_id(contract_name: &str) -> u64 {
        let chain = Self::get().lock().unwrap();
        chain.cosm_orc.contract_map.code_id(contract_name).unwrap()
    }

    // returns the deployed address for the given contract name
    pub fn contract_addr(contract_name: &str) -> String {
        let chain = Self::get().lock().unwrap();
        chain.cosm_orc.contract_map.address(contract_name).unwrap()
    }

    pub fn add_contract_addr(contract_name: &str, contract_addr: &str) {
        let mut chain = Self::get().lock().unwrap();
        chain
            .cosm_orc
            .contract_map
            .add_address(contract_name, contract_addr.to_string())
            .unwrap()
    }

    // Get a clean testing state by clearing out all configured contract addresses
    pub fn clear_deploys() {
        let mut chain = Self::get().lock().unwrap();
        chain.cosm_orc.contract_map.clear_addresses();
    }

    #[track_caller]
    pub fn process_msgs<X, Y, Z>(
        contract_name: &str,
        msgs: &[WasmMsg<X, Y, Z>],
    ) -> Result<Vec<Value>>
    where
        X: Serialize,
        Y: Serialize,
        Z: Serialize,
    {
        let mut chain = Self::get().lock().unwrap();
        chain.cosm_orc.process_msgs(contract_name, msgs)
    }

    #[track_caller]
    pub fn process_msg<X, Y, Z>(contract_name: &str, msg: &WasmMsg<X, Y, Z>) -> Result<Value>
    where
        X: Serialize,
        Y: Serialize,
        Z: Serialize,
    {
        let mut chain = Self::get().lock().unwrap();
        chain.cosm_orc.process_msg(contract_name, msg)
    }

    #[track_caller]
    pub fn store_contracts() {
        let mut chain = Self::get().lock().unwrap();

        let contract_dir = chain.contract_dir.clone();
        chain
            .cosm_orc
            .store_contracts(contract_dir.as_str())
            .expect("error storing contracts");
    }

    pub fn save_gas_report() {
        let chain = Self::get().lock().unwrap();

        let reports = chain
            .cosm_orc
            .profiler_reports()
            .expect("error fetching profile reports");

        let j: Value = serde_json::from_slice(&reports[0].json_data).unwrap();
        fs::write(chain.gas_report_out.clone(), j.to_string()).unwrap();
    }
}
