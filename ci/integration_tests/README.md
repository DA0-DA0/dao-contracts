# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

`cd ci/integration_tests && cargo t` to run all tests.

`cargo t fn_test_name` to run individual integration tests.

## Running Locally

### Hitting Local Juno
* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `cd ci/integration_tests`
* `SKIP_CONTRACT_STORE=true GAS_OUT_DIR="gas_reports" CONFIG="configs/local.yaml" cargo t`

### Hitting Testnet
* `cd ci/integration_tests`
* Change `src/helpers/chain.rs::test_account()` with your testnet account
* `SKIP_CONTRACT_STORE=true GAS_OUT_DIR="gas_reports" CONFIG="configs/testnet.yaml" cargo t`


## Adding New Integration Tests

Add new tests in `src/tests`:
```rust
#[test_context(Chain)]
#[test]
fn new_dao_has_no_items(chain: &mut Chain) {
    let key: SigningKey = chain.key.clone().try_into().unwrap();
    let account = key
        .public_key()
        .account_id(&chain.cfg.chain_cfg.prefix)
        .unwrap();

    let res = create_dao(
        chain, 
        Some(account.to_string()), 
        "ex_create_dao", 
        account.to_string()
    );
    assert!(res.is_ok());
    let dao = res.unwrap();

    // use the native rust types to interact with the contract
     let res = chain
        .orc
        .query(
            "cw_core",
            "exc_items_get",
            &cw_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = serde_json::from_slice(res.data.unwrap().value()).unwrap();

    assert_eq!(res.item, None);
}
```

Run `cargo c` to compile the tests.
