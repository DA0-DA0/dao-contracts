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
    pub fn deploy_code_id(contract_name: &str) -> u64 {
        let chain = Self::get().lock().unwrap();

        chain
            .cosm_orc
            .contract_map
            .get(contract_name)
            .expect("contract not stored")
            .code_id
    }

    // returns the deployed code address for the given contract name
    pub fn deploy_code_addr(contract_name: &str) -> String {
        let chain = Self::get().lock().unwrap();

        chain
            .cosm_orc
            .contract_map
            .get(contract_name)
            .expect("contract not stored")
            .address
            .clone()
            .expect("contract not deployed")
    }

    pub fn add_deploy_code_addr(contract_name: &str, contract_addr: &str) {
        let mut chain = Self::get().lock().unwrap();

        chain
            .cosm_orc
            .contract_map
            .get_mut(contract_name)
            .unwrap()
            .address = Some(contract_addr.to_string())
    }

    // Get a clean testing state by clearing out all configured contract addresses
    pub fn clear_deploys() {
        let mut chain = Self::get().lock().unwrap();

        for (_contract, deploy) in chain.cosm_orc.contract_map.iter_mut() {
            deploy.address = None
        }
    }

    #[track_caller]
    pub fn process_msgs<X, Y, Z>(
        contract_name: String,
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
    pub fn process_msg<X, Y, Z>(contract_name: String, msg: &WasmMsg<X, Y, Z>) -> Result<Value>
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
            .unwrap();
    }

    pub fn save_gas_report() {
        let chain = Self::get().lock().unwrap();

        let reports = chain.cosm_orc.profiler_reports().unwrap();

        let j: Value = serde_json::from_slice(&reports[0].json_data).unwrap();
        fs::write(chain.gas_report_out.clone(), j.to_string()).unwrap();
    }
}
