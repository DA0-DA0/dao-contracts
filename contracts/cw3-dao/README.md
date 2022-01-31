# DAO DAO

**NOT PRODUCTION READY**

This builds on [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) and instead has the voting set maintained by cw20 tokens. This allows for the cw20s to serve as governance tokens in the DAO, similar to how governance tokens work using contracts like [Compound Governance](https://compound.finance/governance).

## Execution Process

First, a voter with an cw20 balance must submit a proposal. The proposer can set an expiration time for the voting process, or it defaults to the limit provided when creating the contract (so proposals can be closed after several days).

Before the proposal has expired, any voter with the required cw20 token can add their vote. Only "Yes" votes are tallied. If enough "Yes" votes were submitted before the proposal expiration date, the status is set to "Passed".

Once a proposal is "Passed", anyone with the correct cw20 token may submit an "Execute" message. This will trigger the proposal to send all stored messages from the proposal and update it's state to "Executed", so it cannot run again. (Note if the execution fails for any reason - out of gas, insufficient funds, etc - the state update will be reverted, and it will remain "Passed", so you can try again).

Once a proposal has expired without passing, anyone can submit a "Close"
message to mark it closed. This has no effect beyond cleaning up the UI/database.

## Complimentary Contracts

These contracts can be used in combination with the cw-dao contract to extend functionality. Simply make a proposal to instantiate them via the cw-dao.

- [cw-vest](https://github.com/ben2x4/cw-vest): Adds the ability to vest cw-20 tokens. Useful if you want community members to be incentivized over a longer term or for payments on projects that have multiple phases.
- [cw3-multisig](../cw3-multisig): Can be used to add SubDAO functionality. Create teams and assign them budgets.
- [cw20-escrow](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw20-escrow): Can be used for setting aside funds intended for a bounty. DAO votes to create a bounty via the cw20-escrow contract and again to payout the bounty when someone submits a claim proposal.

## Testing

You will need Rust 1.58.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

## Deploying in production

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw_dao.wasm .
ls -l cw3_dao.wasm
sha256sum cw3_dao.wasm

cp ../../target/wasm32-unknown-unknown/release/cw20_gov.wasm .
ls -l cw20_gov.wasm
sha256sum cw20_gov.wasm
```

Or for a production-ready (optimized) build, run a build command in the repository root: https://github.com/CosmWasm/cosmwasm-plus#compiling.

You can then upload the contract code to the blockchain (which only needs to be done once) and instantiate the contract. (Further documentation coming soon).
