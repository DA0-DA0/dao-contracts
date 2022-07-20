# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

## Running Locally

* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `tail -n 1 default-account.txt | junod keys add localval --recover`
* `cd ci/integration_tests`
* Uncomment `code_ids` section in `config.yaml` (TODO: Make this easier for local dev)
* `CONTRACT_DIR="../../artifacts" GAS_REPORT_OUT="gas_report.json" RUST_LOG=debug cargo test`
