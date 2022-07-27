use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::{config::cfg::Config, profilers::gas_profiler::GasProfiler};
use ctor::ctor;
use once_cell::sync::OnceCell;
use rand::Rng;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use test_context::TestContext;

static CONFIG: OnceCell<Config> = OnceCell::new();

pub struct Chain {
    pub orc: CosmOrc,
    gas_report_dir: String,
}

// TODO: Make tests run in parallel
// Im getting the following cosmos-sdk error when running in parallel right now:
//   `account sequence mismatch, expected 92, got 91: incorrect account sequence`

impl TestContext for Chain {
    fn setup() -> Self {
        let gas_report_dir = env::var("GAS_OUT_DIR").expect("missing GAS_OUT_DIR env var");

        let cfg = CONFIG.get().unwrap().clone();
        let orc = CosmOrc::new(cfg).add_profiler(Box::new(GasProfiler::new()));
        Self {
            orc,
            gas_report_dir,
        }
    }

    fn teardown(self) {
        // save gas report for this test:
        let reports = self
            .orc
            .profiler_reports()
            .expect("error fetching profile reports");

        let j: Value = serde_json::from_slice(&reports[0].json_data).unwrap();

        let p = Path::new(&self.gas_report_dir);
        if !p.exists() {
            fs::create_dir(p).unwrap();
        }

        let mut rng = rand::thread_rng();
        let file_name = format!("test-{}.json", rng.gen::<u32>());
        fs::write(p.join(file_name), j.to_string()).unwrap();
    }
}

#[ctor]
fn global_setup() {
    env_logger::init();
    let contract_dir = env::var("CONTRACT_DIR").expect("missing CONTRACT_DIR env var");
    let config = env::var("CONFIG").expect("missing yaml CONFIG env var");

    let mut cfg = Config::from_yaml(&config).unwrap();
    let mut orc = CosmOrc::new(cfg.clone()).add_profiler(Box::new(GasProfiler::new()));

    orc.store_contracts(&contract_dir).unwrap();

    // persist stored code_ids in CONFIG, so we can reuse for all tests
    cfg.code_ids = orc
        .contract_map
        .deploy_info()
        .iter()
        .map(|(k, v)| (k.clone(), v.code_id))
        .collect();

    CONFIG.set(cfg).expect("error initializing Config");
}
