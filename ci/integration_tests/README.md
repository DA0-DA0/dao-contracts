# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

`cd ci/integration_tests && cargo t` to run all tests.

`cargo t fn_test_name` to run individual integration tests.

## Running Locally

TODO: Add a just file to make this all easy

### Hitting Local Juno
* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `tail -n 1 default-account.txt | junod keys add localval --recover`
* `cd ci/integration_tests`
* `SKIP_CONTRACT_STORE=true GAS_OUT_DIR="gas_reports" CONFIG="configs/local.yaml" RUST_LOG=debug cargo t`

### Hitting Testnet
* `cd ci/integration_tests`
* Configure `configs/testnet.yaml` with your junod testnet key name
* `SKIP_CONTRACT_STORE=true GAS_OUT_DIR="gas_reports" CONFIG="configs/testnet.yaml" RUST_LOG=debug cargo t`


## Adding New Integration Tests

Add new tests in `src/tests`:
```rust
#[test_context(Chain)]
#[test]
fn new_dao_has_no_items(chain: &mut Chain) {
    let dao = create_dao(chain, Some(admin_addr.clone()), "ex_create_dao", admin_addr);

    // use the native rust types to interact with the contract
    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
        key: "foobar".to_string(),
    });

    // NOTE: `cw_core` wasm was stored in test_harness::test_runner.rs:setup()
    let res = chain.orc.process_msg("cw_core", "ex_get_item", &msg).unwrap();
    let res: GetItemResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(res.item, None);
}
```

Run `cargo c` to compile the tests.
