# Contributing to DAO DAO

Thanks for your interest in contributing. We value kindness, humility,
and low-ego communication. You can read our [Code of
Conduct](./CODE_OF_CONDUCT.md) for a poetic version of this.

There are many ways you can contribute.

- If you're interested in doing smart contract work, you're in the
  right place.
- If you want to work on our UI, check out the [dao-dao-ui
  repo](https://github.com/DA0-DA0/dao-dao-ui).
- If you want to contribute documentation, check out our [docs
  repo](https://github.com/DA0-DA0/docs).
- If you want to contribute thoughts, feedback, or ideas join the [DAO
  DAO Discord](https://discord.gg/sAaGuyW3D2).

## Getting started

To work on the DAO DAO smart contracts you'll need a reasonable
understanding of the Rust programming language. The [Rust
book](https://doc.rust-lang.org/book/) is a really good place to pick
this up.

Before picking up any issues, you may also want to read our [design
wiki
page](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design)
which gives an overview of how all of the DAO DAO smart contracts fit
together.

Finally, consider reading our [security best
practices](https://github.com/DA0-DA0/dao-contracts/wiki/CosmWasm-security-best-practices)
wiki page. We won't land anything that doesn't follow those
guidelines.

### Install Dev Dependencies

- [Rust](https://doc.rust-lang.org/book/ch01-01-installation.html)
- [Just](https://github.com/casey/just#packages)
- [Yarn](https://yarnpkg.com/)
- [Docker](https://docs.docker.com/engine/install/)
- [jq](https://stedolan.github.io/jq/download/)
- [node](https://nodejs.org/en/download/) (>= v15)

Our development workflow is just like a regular Rust project:

- `cargo build` from the repository root to build the contracts.
- `cargo test` from the repository root to test the contracts.

To build WASM files for deploying on a blockchain, run:

```sh
just workspace-optimize
```

from the repository root.

**NOTE**: If you are using VSCode and are seeing rust-analyzer "proc macro not expanded errors", you may need to add the following to your `settings.json`:

```json
"rust-analyzer.procMacro.enable": false
```

## Getting ready to make a PR

Before making a PR, you'll need to do two things to get CI passing:

1. Generate schema files for the contracts.
2. Generate Typescript interfaces from those schemas.

You can do both of these by running:
```sh
just gen
```

### Generating schema files

We generate JSON schema files for all of our contracts' query and
execute messages. We use those schema files to generate Typescript
code for interacting with those contracts via
[ts-codegen](https://github.com/CosmWasm/ts-codegen). This generated
Typescript code is then [used in the
UI](https://github.com/DA0-DA0/dao-dao-ui/tree/40f3cbfe676a98bf7b9db7b646e74e5b2dae4502/packages/state/clients)
to interact with our contracts. Generating this code means that
frontend developers don't have to manually write query and execute
messages for each method on our contracts.

To make sure these files are generated correctly, make sure to add any
new data types used in contract messages to `examples/schema.rs`.

If you are adding a new query, ts-codegen expects:

1. There is a corresponding query response type exported from
   `examples/schema.rs` for that contract.
2. The query response has a name in the form `<queryName>Response`.

For example, if you added a `ListStakers {}` query, you'd also need to
make sure to export a type from the schema file called
`ListStakersResponse`.

Most of the time, this will just be a struct with the same
name. Occasionally though you may have queries that return types like
`Vec<Addr>`. In these cases you'll still need to export a type from
the schema file for this. You can do so with:

```rust
export_schema_with_title(&schema_for!(Vec<Addr>), &out_dir, "Cw20TokenListResponse");
```

Once you have exported these types, you can generate schema files for
all the contracts by running:

```sh
just gen-schema
```

### Generating the Typescript interface

To generate the Typescript interface, after generating the schema
files, run:

```sh
just gen-typescript
```

To do this you'll need [yarn](https://yarnpkg.com/) installed.

If you get errors complaining about a missing query response type it
is likely because you forgot to export that type from
`examples/schema.rs` for that contract.

## Deploying in a development environment

Build and deploy the contracts to a local chain running in Docker with:

```sh
just bootstrap-dev
```

> Note: These juno accounts are from the [test
> accounts](ci/configs/test_accounts.json), which you can use for testing (DO NOT
> store any real funds with these accounts). You can add more juno
> account addresses you wish to test here.

This will run a chain locally in a docker container, then build and
deploy the contracts to that chain.

The script will output something like:

```sh
NEXT_PUBLIC_CW20_CODE_ID=1
NEXT_PUBLIC_CW4GROUP_CODE_ID=2
NEXT_PUBLIC_CWCORE_CODE_ID=7
NEXT_PUBLIC_CWPROPOSALSINGLE_CODE_ID=11
NEXT_PUBLIC_CW4VOTING_CODE_ID=5
NEXT_PUBLIC_CW20STAKEDBALANCEVOTING_CODE_ID=4
NEXT_PUBLIC_STAKECW20_CODE_ID=13
NEXT_PUBLIC_DAO_CONTRACT_ADDRESS=juno1zlmaky7753d2fneyhduwz0rn3u9ns8rse3tudhze8rc2g54w9ysqgjt23l
NEXT_PUBLIC_V1_FACTORY_CONTRACT_ADDRESS=juno1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucssml9ug
```

This output can be directly copied and pasted into a `.env` file in the
[DAO DAO UI](https://github.com/DA0-DA0/dao-dao-ui) for local
development.

Note, to send commands to the docker container:

```sh
docker exec -i cosmwasm junod status
```

Some commands require a password which defaults to `xxxxxxxxx`. You can use them like so:

```sh
echo xxxxxxxxx | docker exec -i cosmwasm  junod keys show validator -a
```
