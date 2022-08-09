# CW-Vest

This contract allows you to establish a schedule of payments of native and/or cw20 tokens. This sequence is immutable once the contract is instantiated. Calling the pay function will execute all vested payments.

## Instantiation

Instantiating the contract takes an array of payments. These payments have an expiry field which defines when it is eligible for payment. The payment currency can be either a cw20 token defined by a token address or a native token.

## Funding the contract
After instantiation the contract must be funded with the exact amount required to make the scheduled payments. This means sending the cw20 and native tokens to the contract address directly. No special method needs to be called to fund the contract. There are currently very little safeguards, and it is important the instantiator funds the exact sum of the payment amounts.

## Making Payments
When payments are vested, any caller can trigger the pay function which will execute all unpaid vested payments.

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw_dao.wasm .
ls -l cw_dao.wasm
sha256sum cw_dao.wasm
```

Or for a production-ready (optimized) build, run a build command in
the repository root: https://github.com/CosmWasm/cosmwasm-plus#compiling.

