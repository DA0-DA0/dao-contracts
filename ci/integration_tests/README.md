# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

`cd ci/integration_tests && cargo t` to run all tests.

`cargo t fn_test_name` to run individual integration tests.

## Running Locally

### Hitting Local Juno

* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `cd ci/integration_tests`
* `CONFIG="configs/local.yaml" cargo t`

### Hitting Testnet

* `cd ci/integration_tests`
* Change `src/helpers/chain.rs::test_account()` with your testnet account
* `CONFIG="configs/testnet.yaml" cargo t`


### Skipping Contract Storage

By default all of the smart contracts are stored on-chain once before all of the tests are run. 
This is time consuming when writing tests. If you want to skip this step you can use the `SKIP_CONTRACT_STORE=true` flag like so:

`SKIP_CONTRACT_STORE=true GAS_OUT_DIR="gas_reports" CONFIG="configs/local.yaml" cargo t`

This requires the `code ids` stored in [`configs/local.yaml`](configs/local.yaml) to be set to the correct, up to date value.
For now you can see the output from `scripts/deploy_local.sh` and manually copy them over.
These values change as contracts are added, so this is likely out of date, and a more robust solution for this is needed.


## Adding New Integration Tests

Add new tests in `src/tests`:
```rust
#[test_context(Chain)]
#[test]
#[ignore]
fn new_dao_has_no_items(chain: &mut Chain) {
    let res = create_dao(
        chain, 
        Some(chain.user.addr.clone()),
        "ex_create_dao", 
       chain.user.addr.clone()
    );
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
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);
}
```

We are currentlying [ignoring](https://doc.rust-lang.org/book/ch11-02-running-tests.html#ignoring-some-tests-unless-specifically-requested) all integration tests by adding the `#[ignore]` annotation to them, because we want to skip them when people run `cargo test` from the workspace root.

Run `cargo c` to compile the tests.
