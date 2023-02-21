# DAO Contracts

[![codecov](https://codecov.io/gh/DA0-DA0/dao-contracts/branch/main/graph/badge.svg?token=SCKOIPYZPV)](https://codecov.io/gh/DA0-DA0/dao-contracts)

This is a collection of smart contracts for building composable, modular, and upgradable DAOs.

For a detailed look at how these contracts work, see [our wiki](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design).

Our most recently [audited](https://github.com/oak-security/audit-reports/blob/master/DAO%20DAO/2023-02-06%20Audit%20Report%20-%20DAO%20DAO%202%20v1.0.pdf) release is `v2.0.0`. If you believe you have found a problem, please [let us know](SECURITY.md).

## Overview

Every DAO is made up of three modules:

1. A voting power module, which manages the voting power of DAO members.
2. Any number of proposal modules, which manage proposals in the DAO.
3. A core module, which holds the DAO treasury.

![image](https://user-images.githubusercontent.com/30676292/220181882-737c4dd3-a85d-498c-a1f2-067b317418a9.png)

For example, voting power might be based on [staked governance tokens](https://github.com/DA0-DA0/dao-contracts/tree/main/contracts/voting/dao-voting-cw20-staked), [staked NFTs](https://github.com/DA0-DA0/dao-contracts/tree/main/contracts/voting/dao-voting-cw721-staked), or [membership](https://github.com/DA0-DA0/dao-contracts/tree/main/contracts/voting/dao-voting-cw4) and proposal modules might implement [yes/no](https://github.com/DA0-DA0/dao-contracts/tree/main/contracts/proposal/dao-proposal-single), [multiple-choice](https://github.com/DA0-DA0/dao-contracts/tree/main/contracts/proposal/dao-proposal-multiple), or [ranked-choice](https://github.com/DA0-DA0/dao-contracts/tree/main/contracts/proposal/dao-proposal-condorcet) voting.

Each module type has a [standard interface](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design). As a result, any voting module can be used with any proposal module, and any proposal module with any voting module.

The best way to get started is to create a DAO! We maintain an [open source](https://github.com/DA0-DA0/dao-dao-ui) frontend you can find at [daodao.zone](https://daodao.zone).

## Why?

Our institutions grew rapidly after 1970, but as time passed their priorities shifted from growth, to protectionism. We're fighting this. We believe The Internet is where the organizations of tomorrow will be built.

DAO DAO is a global community working on Internet governance, and [a real DAO](https://daodao.zone/dao/juno10h0hc64jv006rr8qy0zhlu4jsxct8qwa0vtaleayh0ujz0zynf2s2r7v8q#proposals). We've never raised money, and all our work is open-source. We hope you'll [join us](https://discord.gg/sAaGuyW3D2).

## Links and Resources

- [DAO DAO DAO](https://daodao.zone/dao/juno10h0hc64jv006rr8qy0zhlu4jsxct8qwa0vtaleayh0ujz0zynf2s2r7v8q)
- [Discord](https://discord.gg/sAaGuyW3D2)
- [Docs](https://docs.daodao.zone)
- [Manually Instantiating a DAO](https://github.com/DA0-DA0/dao-contracts/wiki/Instantiating-a-DAO)
- [Twitter](https://github.com/DA0-DA0)
- [What is a DAO?](https://docs.daodao.zone/docs/introduction/what-is-dao)

## Developers

Information about our development workflow and how to contribute can be found in [CONTRIBUTING.md](./CONTRIBUTING.md).

## Testing

### Unit tests

Run `cargo test`, or `just test` from the project root to run the unit tests.

### Integration tests

Run `just bootstrap-dev` to spin up a local environment and `just integration-test-dev` to run tests against it.

See [ci/integration-tests/README.md](ci/integration-tests/README.md) for more information.

## Disclaimer

DAO DAO TOOLING IS PROVIDED “AS IS”, AT YOUR OWN RISK, AND WITHOUT
WARRANTIES OF ANY KIND. No developer or entity involved in creating
the DAO DAO UI or smart contracts will be liable for any claims or
damages whatsoever associated with your use, inability to use, or your
interaction with other users of DAO DAO tooling, including any direct,
indirect, incidental, special, exemplary, punitive or consequential
damages, or loss of profits, cryptocurrencies, tokens, or anything
else of value.
