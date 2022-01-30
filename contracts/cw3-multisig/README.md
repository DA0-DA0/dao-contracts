# CW3 Multisig

This is a DAO DAO fork of [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) which builds on [cw3-fixed-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-fixed-multisig) with a more powerful implementation of the [cw3 spec](https://github.com/CosmWasm/cw-plus/tree/main/packages/cw3). It is a multisig contract that is backed by a [cw4 (group)](https://github.com/CosmWasm/cw-plus/tree/main/packages/cw4) contract, which independently maintains the voter set.

In addition to the dynamic voting set, the main difference with the native Cosmos SDK multisig, is that it aggregates the signatures on chain, with visible proposals (like `x/gov` in the Cosmos SDK), rather than requiring signers to share signatures off chain.

## Execution Process

First, a registered voter (a member of the multisig) must submit a proposal. This also includes the first "Yes" vote on the proposal by the proposer. The proposer can set an expiration time for the voting process, or it defaults to the limit provided when creating the contract (so proposals can be closed after several days).

Before the proposal has expired, any voter with non-zero weight can add their vote. Only "Yes" votes are tallied. If enough "Yes" votes were submitted before the proposal expiration date, the status is set to "Passed".

Once a proposal is "Passed", anyone may submit an "Execute" message. This will trigger the proposal to send all stored messages from the proposal and update it's state to "Executed", so it cannot run again. (Note if the execution fails for any reason - out of gas, insufficient funds, etc - the state update will be reverted, and it will remain "Passed", so you can try again).

Once a proposal has expired without passing, anyone can submit a "Close" message to mark it closed. This has no effect beyond cleaning up the UI/database.

## Running this contract

You will need Rust 1.58.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw3_fixed_multisig.wasm .
ls -l cw3_fixed_multisig.wasm
sha256sum cw3_fixed_multisig.wasm
```

Or for a production-ready (optimized) build, run a build command in
the repository root: https://github.com/CosmWasm/cw-plus#compiling.
