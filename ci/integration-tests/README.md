# Dao Dao Integration Tests

Dao Dao e2e integration tests with gas profiling.

`cd ci/integration_tests && cargo t` to run all tests.

`cargo t fn_test_name` or `just integration-test-dev fn_test_name` to run individual integration tests.

## Running Locally

### Hitting Local Juno

#### Run All Tests
`just integration-test`

#### Nicest Test Dev Loop

This will create a local dev env, and then easily test one integration test, skipping optimization + contract storage each time we call `just integration-test-dev`.

Run once to init env:
* `just bootstrap-dev`

Run many times while developing tests:
* `just integration-test-dev fn_test_name`

Or Use `just integration-test-dev` to run all integration tests while skipping setting up local dev + contract optimization / storage.

### Hitting Testnet

* `cd ci/integration_tests`
* Change `src/helpers/chain.rs::test_account()` with your testnet account
* `CONFIG="../configs/cosm-orc/testnet.yaml" just integration-test`


## Adding New Integration Tests

Add new tests in `src/tests`:
```rust
#[test_context(Chain)]
#[test]
#[ignore]
fn new_dao_has_no_items(chain: &mut Chain) {
    let res = create_dao(
        chain,
        None,
        "ex_create_dao",
        chain.users["user1"].account.address.clone(),
    );
    let dao = res.unwrap();

    // use the native rust types to interact with the contract
     let res = chain
        .orc
        .query(
            "cw_core",
            &cwd_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);
}
```

We are currentlying
[ignoring](https://doc.rust-lang.org/book/ch11-02-running-tests.html#ignoring-some-tests-unless-specifically-requested)
all integration tests by adding the `#[ignore]` annotation to them,
because we want to skip them when people run `cargo test` from the
workspace root.

Run `cargo c` to compile the tests.
