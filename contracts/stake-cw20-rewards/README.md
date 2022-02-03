# Stake CW20 rewards

This is a basic implementation of a cw20 staking rewards contract. It is used to provide rewards to stakers in the `stake-cw20` contract.

## Running this contract

You will need Rust 1.58.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/stake_cw20.wasm .
ls -l stake_cw20.wasm
sha256sum stake_cw20.wasm
```

Or for a production-ready (optimized) build, run a build command in the the repository root: https://github.com/CosmWasm/cw-plus#compiling.
