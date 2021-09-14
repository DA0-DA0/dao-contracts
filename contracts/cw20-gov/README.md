# CW20 Gov

This is a basic implementation of a cw20 governance contract. It implements
the CW20 spec with the addition of a BalanceAtHeight Query Message and
is designed to be deployed as is, or imported into other contracts to easily build
cw20-compatible tokens with custom logic.

Inspired by the design of the Compound governance token, `BalanceAtHeight`
can be used to get a users balance at the start of a governance proposal.

Implements:

- [x] CW20 Base
- [x] Mintable extension
- [x] Allowances extension
- [x] BalanceAtHeight QueryMsg
## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw20_gov.wasm .
ls -l cw20_gov.wasm
sha256sum cw20_gov.wasm
```

Or for a production-ready (optimized) build, run a build command in the
the repository root: https://github.com/CosmWasm/cw-plus#compiling.

## Importing this contract

You can also import much of the logic of this contract to build another
ERC20-contract, such as a bonding curve, overiding or extending what you
need.

Basically, you just need to write your handle function and import 
`cw20_gov::contract::handle_transfer`, etc and dispatch to them.
This allows you to use custom `ExecuteMsg` and `QueryMsg` with your additional
calls, but then use the underlying implementation for the standard cw20
messages you want to support. The same with `QueryMsg`. You *could* reuse `instantiate`
as it, but it is likely you will want to change it. And it is rather simple.
