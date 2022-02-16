# DAO DAO

**NOT PRODUCTION READY**

## Testing

You will need Rust 1.58.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

## Deploying in production

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw3_multiple_choice.wasm .
ls -l cw3_multiple_choice.wasm
sha256sum cw3_multiple_choice.wasm
```

Or for a production-ready (optimized) build, run a build command in the repository root: https://github.com/CosmWasm/cosmwasm-plus#compiling.

You can then upload the contract code to the blockchain (which only needs to be done once) and instantiate the contract. (Further documentation coming soon).
