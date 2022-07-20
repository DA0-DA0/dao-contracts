use anyhow::Result;
use cosm_orc::orchestrator::cosm_orc::{CosmOrc, DeployInfo, WasmMsg};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;

static COSM_ORC: OnceCell<Mutex<CosmOrc>> = OnceCell::new();

// Chain gives all the tests access to a global CosmOrc singleton
pub struct Chain {}
impl Chain {
    pub fn init(cosm_orc: CosmOrc) {
        COSM_ORC.set(Mutex::new(cosm_orc));
    }

    fn get() -> &'static Mutex<CosmOrc> {
        COSM_ORC.get().expect("cosm-orc is not initialized")
    }

    // returns the deployed code_id for the given contract name
    pub fn deploy_code_id(contract_name: &str) -> u64 {
        let cosm_orc = Self::get().lock().unwrap();

        cosm_orc
            .contract_map
            .get(contract_name)
            .expect("contract not stored")
            .code_id
    }

    // returns the deployed code address for the given contract name
    pub fn deploy_code_addr(contract_name: &str) -> String {
        let cosm_orc = Self::get().lock().unwrap();

        cosm_orc
            .contract_map
            .get(contract_name)
            .expect("contract not stored")
            .address
            .clone()
            .expect("contract not deployed")
    }

    pub fn add_deploy_code_addr(contract_name: &str, contract_addr: &str) {
        let mut cosm_orc = Self::get().lock().unwrap();

        cosm_orc
            .contract_map
            .get_mut(contract_name)
            .unwrap()
            .address = Some(contract_addr.to_string())
    }

    // Get a clean testing state by clearing out all configured contract addresses
    pub fn clear_deploys() {
        let mut cosm_orc = Self::get().lock().unwrap();

        for (_contract, deploy) in cosm_orc.contract_map.iter_mut() {
            deploy.address = None
        }
    }

    pub fn process_msgs<X, Y, Z>(
        contract_name: String,
        msgs: &[WasmMsg<X, Y, Z>],
    ) -> Result<Vec<Value>>
    where
        X: Serialize,
        Y: Serialize,
        Z: Serialize,
    {
        let mut cosm_orc = Self::get().lock().unwrap();
        cosm_orc.process_msgs(contract_name, msgs)
    }

    pub fn process_msg<X, Y, Z>(contract_name: String, msg: &WasmMsg<X, Y, Z>) -> Result<Value>
    where
        X: Serialize,
        Y: Serialize,
        Z: Serialize,
    {
        let mut cosm_orc = Self::get().lock().unwrap();
        cosm_orc.process_msg(contract_name, msg)
    }
}
