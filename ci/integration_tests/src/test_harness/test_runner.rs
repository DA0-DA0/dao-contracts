extern crate test;

use cosm_orc::{
    config::cfg::Config, orchestrator::cosm_orc::CosmOrc, profilers::gas_profiler::GasProfiler,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::panic;
use std::time::Instant;
use test::TestDescAndFn;

use super::chain::Chain;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Status {
    Ok,
    Fail,
}

// Runs all integration tests in one thread
pub fn cosm_orc_test_runner(tests: &[&TestDescAndFn]) {
    setup();

    println!("running {} integration tests", tests.len());

    let mut passed = 0;
    let mut failed = 0;
    let time = Instant::now();

    for &test in tests {
        if let test::TestFn::StaticTestFn(f) = test.testfn {
            let res = panic::catch_unwind(f);
            if res.is_err() {
                failed += 1;
            } else {
                passed += 1;
            }
        } else {
            todo!()
        }

        // create a clean slate for next test
        Chain::clear_deploys();
    }

    let status = if failed == 0 {
        Status::Ok
    } else {
        Status::Fail
    };

    println!(
        "{}",
        format!("\ntest result: {:?}. {} passed; {} failed; 0 ignored; 0 measured; 0 filtered out; finished in {:.2?}", status, passed, failed, time.elapsed())
    );

    teardown();

    if status == Status::Fail {
        std::process::exit(1);
    }
}

fn setup() {
    env_logger::init();
    let contract_dir = env::var("CONTRACT_DIR").expect("missing CONTRACT_DIR env var");
    let gas_report_out = env::var("GAS_REPORT_OUT").expect("missing GAS_REPORT_OUT env var");

    let cfg = Config::from_yaml("config.yaml").unwrap();
    let mut cosm_orc = CosmOrc::new(cfg).add_profiler(Box::new(GasProfiler::new()));

    Chain::init(cosm_orc, contract_dir, gas_report_out);

    Chain::store_contracts();
}

fn teardown() {
    Chain::save_gas_report();
}
