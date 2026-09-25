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
            &dao_interface::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);
}
```

We are currently
[ignoring](https://doc.rust-lang.org/book/ch11-02-running-tests.html#ignoring-some-tests-unless-specifically-requested)
all integration tests by adding the `#[ignore]` annotation to them,
because we want to skip them when people run `cargo test` from the
workspace root.

Run `cargo c` to compile the tests.

## Native client configuration

Integration tests and `bootstrap-env` now share the native
[`dao-chain-client`](../chain-client/README.md) facade. Configuration paths and
code-ID/address maps are unchanged, but `chain_cfg.rpc_endpoint` is required.
The local CI configuration uses `http://127.0.0.1:26657`. An existing
`grpc_endpoint` is preserved in YAML but no longer used by this client.
For a custom configuration, supply the matching chain's RPC endpoint yourself;
do not substitute a gRPC URL, change the chain ID to fit an unrelated endpoint,
or reuse historical deployment IDs on a different chain. The fixture client
validates the chain ID and requires CometBFT 0.37 or 0.38. The local fixture
runs Juno v28.0.2 and funds fees in `ujuno`; the historical `uni-5` testnet
configuration is unchanged and still expects its own endpoint and `ujunox`.

Transactions are submitted once and require committed execution success, not
just CheckTx admission. An ambiguous timeout is not permission to resubmit.
Failed global setup is cached so later tests cannot accidentally repeat uploads.
Tests remain single-threaded because the fixture accounts are shared. Contract
messages, application assertions, fee calculation and gas-report format are
preserved; native-client changes do not upgrade the contract or Test Tube runtime.
