# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

`cargo test` to run all tests.

`cargo test fn_test_name` to run individual integration tests.

## Running Locally

TODO: Add a just file to make this all easy

### Hitting Local Juno
* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `tail -n 1 default-account.txt | junod keys add localval --recover`
* `cd ci/integration_tests`
* `CONTRACT_DIR="../../artifacts" GAS_REPORT_OUT="gas_report.json" CONFIG="configs/local.yaml" RUST_LOG=debug cargo test`

### Hitting Testnet
* `cd ci/integration_tests`
* Configure `configs/testnet.yaml` with your junod testnet key name
* `CONTRACT_DIR="../../artifacts" GAS_REPORT_OUT="gas_report.json" CONFIG="configs/testnet.yaml" RUST_LOG=debug cargo test`


## Adding New Integration Tests

Add new tests in `src/tests`:
```rust
#[test]
fn new_dao_has_no_items() {
     let dao = create_dao(Some(admin_addr.clone()), "ex_create_dao", admin_addr);

    // use the native rust types to interact with the contract
    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
        key: "foobar".to_string(),
    });

    // NOTE: `cw_core` wasm was stored in test_harness::test_runner.rs:setup()
    let res = Chain::process_msg("cw_core", "ex_get_item", &msg).unwrap();
    let res: GetItemResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(res.item, None);
}
```

Run `cargo c` to compile the tests.
