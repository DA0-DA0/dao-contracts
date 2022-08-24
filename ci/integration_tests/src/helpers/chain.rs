use cosm_orc::config::key::Key;
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::{
    config::cfg::Config, config::key::SigningKey, profilers::gas_profiler::GasProfiler,
};
use once_cell::sync::OnceCell;
use rand::Rng;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use test_context::TestContext;

static CONFIG: OnceCell<Cfg> = OnceCell::new();

#[derive(Debug)]
pub struct Cfg {
    cfg: Config,
    gas_report_dir: String,
}

pub struct Chain {
    pub cfg: Config,
    pub orc: CosmOrc,
    pub user: Account,
}

pub struct Account {
    pub addr: String,
    pub key: SigningKey,
}

// NOTE: we have to run the integration tests in one thread right now.
// We get `account sequence mismatch` CosmosSDK error when we run in parallel.
// We could either serialize the `account.sequence` per key, or use a different key per test.

impl TestContext for Chain {
    fn setup() -> Self {
        let cfg = CONFIG.get_or_init(global_setup).cfg.clone();
        let orc = CosmOrc::new(cfg.clone())
            .unwrap()
            .add_profiler(Box::new(GasProfiler::new()));

        let user = test_account(&cfg.chain_cfg.prefix);

        Self { cfg, orc, user }
    }

    fn teardown(self) {
        let cfg = CONFIG.get().unwrap();
        save_gas_report(&self.orc, &cfg.gas_report_dir);
    }
}

fn test_account(prefix: &str) -> Account {
    // TODO: Make this configurable + bootstrap the local env with many test accounts
    let key = SigningKey {
        name: "localval".to_string(),
        key: Key::Mnemonic("siren window salt bullet cream letter huge satoshi fade shiver permit offer happy immense wage fitness goose usual aim hammer clap about super trend".to_string()),
    };
    let addr = key.to_account(prefix).unwrap().to_string();

    Account { addr, key }
}

// global_setup() runs once before all of the tests
fn global_setup() -> Cfg {
    env_logger::init();
    let config = env::var("CONFIG").expect("missing yaml CONFIG env var");
    let gas_report_dir = env::var("GAS_OUT_DIR").unwrap_or_else(|_| "gas_reports".to_string());

    let mut cfg = Config::from_yaml(&config).unwrap();
    let mut orc = CosmOrc::new(cfg.clone())
        .unwrap()
        .add_profiler(Box::new(GasProfiler::new()));

    let account = test_account(&cfg.chain_cfg.prefix);

    let skip_storage = env::var("SKIP_CONTRACT_STORE").unwrap_or_else(|_| "false".to_string());
    if !skip_storage.parse::<bool>().unwrap() {
        let contract_dir = "../../artifacts";
        orc.store_contracts(contract_dir, &account.key).unwrap();
        save_gas_report(&orc, &gas_report_dir);
        // persist stored code_ids in CONFIG, so we can reuse for all tests
        cfg.code_ids = orc
            .contract_map
            .deploy_info()
            .iter()
            .map(|(k, v)| (k.clone(), v.code_id))
            .collect();
    }

    Cfg {
        cfg,
        gas_report_dir,
    }
}

fn save_gas_report(orc: &CosmOrc, gas_report_dir: &str) {
    let reports = orc
        .profiler_reports()
        .expect("error fetching profile reports");

    let j: Value = serde_json::from_slice(&reports[0].json_data).unwrap();

    let p = Path::new(gas_report_dir);
    if !p.exists() {
        fs::create_dir(p).unwrap();
    }

    let mut rng = rand::thread_rng();
    let file_name = format!("test-{}.json", rng.gen::<u32>());
    fs::write(p.join(file_name), j.to_string()).unwrap();
}
