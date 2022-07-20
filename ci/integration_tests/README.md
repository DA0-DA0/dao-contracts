# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

## Running Locally

* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `cd ci/integration_tests`
* Uncomment `code_ids` section in `config.yaml` + change `key_name` (TODO: Make this easier for local dev)
* `CONTRACT_DIR="../../artifacts" RUST_LOG=debug cargo test`
