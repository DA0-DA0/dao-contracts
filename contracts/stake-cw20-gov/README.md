# Stake CW20 Gov

This is a governance implementation of a cw20 staking contract with support for vote delegation. Staked tokens can be unbonded with a configurable unbonding period. Staked balances and voting power can be queried at any arbitrary height by external contracts.

This contract is used to enable DAO voting.

## Running this contract

You will need Rust 1.58.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/stake_cw20_gov.wasm .
ls -l stake_cw20_gov.wasm
sha256sum stake_cw20_gov.wasm
```

Or for a production-ready (optimized) build, run a build command in the the repository root: https://github.com/CosmWasm/cw-plus#compiling.
