# CW3 Multisig

This is a DAO DAO fork of [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) which builds on [cw3-fixed-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-fixed-multisig) with a more powerful implementation of the [cw3 spec](https://github.com/CosmWasm/cw-plus/tree/main/packages/cw3). It is a multisig contract that is backed by a [cw4 (group)](https://github.com/CosmWasm/cw-plus/tree/main/packages/cw4) contract, which independently maintains the voter set.

This provides 2 main advantages:

- You can create two different multisigs with different voting thresholds
  backed by the same group. Thus, you can have a 50% vote, and a 67% vote
  that always use the same voter set, but can take other actions.
- The multisig can be the admin of the group and can vote to add new members

In addition to the dynamic voting set, the main difference with the native Cosmos SDK multisig, is that it aggregates the signatures on chain, with visible proposals (like `x/gov` in the Cosmos SDK), rather than requiring signers to share signatures off chain.

## Instantiation

The first step to create such a multisig is to instantiate a cw4 contract
with the desired member set. For now, this only is supported by
[cw4-group](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw4-group).

If you create a `cw4-group` contract and want a multisig to be able
to modify its own group, do the following in multiple transactions:

- instantiate cw4-group, with your personal key as admin
- instantiate a multisig pointing to the group
- `AddHook{multisig}` on the group contract
- `UpdateAdmin{multisig}` on the group contract

This is the current practice to create such circular dependencies, and depends on an external driver (hard to impossible to script such a self-deploying contract on-chain).

When creating the multisig, you must set the required weight to pass a vote as well as the max/default voting period. (TODO: allow more threshold types)

## Execution Process

First, a registered voter must submit a proposal. This also includes the first "Yes" vote on the proposal by the proposer. The proposer can set an expiration time for the voting process, or it defaults to the limit provided when creating the contract (so proposals can be closed after several days).

Before the proposal has expired, any voter with non-zero weight can add their vote. Only "Yes" votes are tallied. If enough "Yes" votes were submitted before the proposal expiration date, the status is set to "Passed".

Once a proposal is "Passed", anyone may submit an "Execute" message. This will trigger the proposal to send all stored messages from the proposal and update it's state to "Executed", so it cannot run again. (Note if the execution fails for any reason - out of gas, insufficient funds, etc - the state update will be reverted, and it will remain "Passed", so you can try again).

Once a proposal has expired without passing, anyone can submit a "Close" message to mark it closed. This has no effect beyond cleaning up the UI/database.

TODO: this contract currently assumes the group membership is static during the lifetime of one proposal. If the membership changes when a proposal is open, this will calculate incorrect values (future PR).

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
