# cw4-registry

cw4 registry contract indexes group members.

Group admin must register the registry contract address as hook before adding the group to the registry. Membership changes will be executed automatically.

## Running this contract

You will need Rust 1.58.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw4_registry.wasm .
ls -l cw4_registry.wasm
sha256sum cw4_registry.wasm
```

Or for a production-ready (optimized) build, run a build command in the the repository root: https://github.com/CosmWasm/cw-plus#compiling.
