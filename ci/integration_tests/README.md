# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

## Running Locally

* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `tail -n 1 default-account.txt | junod keys add localval --recover`
* `cd ci/integration_tests`
* Uncomment `code_ids` section in `config.yaml` (TODO: Make this easier for local dev)
* `CONTRACT_DIR="../../artifacts" GAS_REPORT_OUT="gas_report.json" RUST_LOG=debug cargo test`

## Adding New Integration Tests

```rust
#[test]
fn new_dao_has_no_items() {
     let dao = create_dao(
        Some(admin_addr.clone()),
        admin_addr,
        None,
        "cw20_staked_balance_voting",
        "cw_proposal_single",
    );

    // use the native rust types to interact with the contract
    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
        key: "foobar".to_string(),
    });

    // NOTE: `cw_core` wasm was stored in test_harness::test_runner.rs:setup()
    let res = Chain::process_msg("cw_core".to_string(), &msg).unwrap();
    let res: GetItemResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(res.item, None);
}
```
