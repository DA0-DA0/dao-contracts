# DAO DAO Contracts

[![codecov](https://codecov.io/gh/DA0-DA0/dao-contracts/branch/main/graph/badge.svg?token=SCKOIPYZPV)](https://codecov.io/gh/DA0-DA0/dao-contracts)

This is a collection of smart contracts for building composable,
modular, and upgradable DAOs.

For an overview of our contract design, see [our
wiki](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design).

| Audited contracts (tag: v1.0.0)                                                | Description                                            |
| :----------------------------------------------------------------------------- | :----------------------------------------------------- |
| [dao-core](contracts/dao-core)                                                 | The core module for a DAO DAO DAO.                     |
| [dao-proposal-single](contracts/proposal/dao-proposal-single)                  | A proposal module for single choice (yes / no) voting. |
| [dao-voting-cw20-staked](contracts/voting/dao-voting-cw20-staked)              | A voting power module for staked governance tokens.    |
| [dao-voting-cw4](contracts/voting/dao-voting-cw4)                              | A voting power module for multisig-style voting.       |
| [cw20-stake](contracts/staking/cw20-stake)                                     | A contract for staking cw20 tokens.                    |
| [cw20-stake-external-rewards](contracts/staking/cw20-stake-external-rewards)   | A contract for providing external staking rewards.     |
| [cw20-stake-reward-distributor](contracts/staking/cw20-stake-reward-distributor) | A contract for distributing rewards via stake-cw20.    |

| Unaudited contracts                                                                      | Description                                                                            |
| :--------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------- |
| [dao-proposal-multiple](contracts/proposal/dao-proposal-multiple)                        | A proposal module for multiple choice proposals.                                       |
| [dao-voting-cw721-staked](contracts/voting/dao-voting-cw721-staked)                      | A voting module based on staked NFTs                                                   |
| [dao-pre-propose-single](contracts/pre-propose/dao-pre-propose-single)                   | A pre-propose module for single choice proposals.                                      |
| [dao-voting-native-staked](contracts/proposal/dao-voting-native-staked)                  | A voting power based on staked native tokens not used to secure the chain e.g. ION.    |
| [dao-voting-staking-denom-staked](contracts/proposal/dao-voting-staking-denom-staked)    | A voting power module based on staked native tokens used to secure the chain e.g. JUNO |
| [dao-pre-propose-multiple](contracts/pre-propose/dao-pre-propose-multiple)               | A pre-propose module for multiple choice proposals.                                    |
| [dao-pre-propose-approval-single](contracts/pre-propose/dao-pre-propose-approval-single) | A pre-propose module for implementing an approval flow.                                |
| [dao-pre-propose-approver](contracts/pre-propose/dao-pre-propose-approver)               | A pre-propose module for automatically creating proposals that need to be approved.    |
| [cw-token-swap](contracts/external/cw-token-swap)                                        | An escrow contract for swapping tokens between DAOs.                                   |
| [cw-vesting](contracts/external/cw-vesting)                                              | A vesting payment contract.                                                            |
| [cw-payroll-factory](contracts/external/cw-payroll-factory)                              | A factory contract for `cw-vesting`.                                                   |

Audited contracts have completed audits by
[securityDAO](https://github.com/securityDAO/audits/blob/7bb8e4910baaea89fddfc025591658f44adbc27c/cosmwasm/dao-contracts/v0.3%20DAO%20DAO%20audit.pdf)
and [Oak
Security](https://github.com/oak-security/audit-reports/blob/2377ba8cfcfd505283c789d706311b06771d6db4/DAO%20DAO/2022-06-22%20Audit%20Report%20-%20DAO%20DAO%20v1.0.pdf)
on the `v1.0.0` tag. An audit for the v2 contracts is forthcoming.

## Packages

| Package                                               | Description                                                                               |
| :---------------------------------------------------- | :---------------------------------------------------------------------------------------- |
| [cw721-controllers](packages/cw721-controllers)       | Manages claims for the [cw721 staking contract](contracts/voting/dao-voting-cw721-staked) |
| [cw-hooks](packages/cw-hooks)                         | Shared hooks functionality.                                                               |
| [dao-interface](packages/dao-interface)               | Provides types and interfaces for interacting with DAO modules.                           |
| [dao-macros](packages/dao-macros)                     | A collection of macros to derive DAO module interfaces on message enums.                  |
| [dao-pre-propose-base](packages/dao-pre-propose-base) | Base package used to implement pre-propose modules.                                       |
| [dao-proposal-hooks](packages/dao-proposal-hooks)     | Interface for managing and dispatching hooks from a proposal module.                      |
| [dao-testing](packages/dao-testing)                   | Common testing functions and types for DAO modules.                                       |
| [dao-vote-hooks](packages/dao-vote-hooks)             | Interface for managing and dispatching vote hooks.                                        |
| [dao-voting](packages/dao-voting)                     | Types and associated methods for handling voting in a CosmWasm DAO.                       |
| [cw-denom](packages/cw-denom)                         | Utilities for working with cw20 and native denoms.                                        |
| [cw-paginate](packages/cw-paginate)                   | Convenience methods for paginating keys and values in a CosmWasm `Map` or `SnapshotMap`.  |

Packages have completed audits by
[securityDAO](https://github.com/securityDAO/audits/blob/7bb8e4910baaea89fddfc025591658f44adbc27c/cosmwasm/dao-contracts/v0.3%20DAO%20DAO%20audit.pdf)
and [Oak
Security](https://github.com/oak-security/audit-reports/blob/2377ba8cfcfd505283c789d706311b06771d6db4/DAO%20DAO/2022-06-22%20Audit%20Report%20-%20DAO%20DAO%20v1.0.pdf)
on the `v1.0.0` tag. An audit for the v2 packages is forthcoming.

## Links and Resources

- [DAO DAO DAO](https://daodao.zone/dao/juno10h0hc64jv006rr8qy0zhlu4jsxct8qwa0vtaleayh0ujz0zynf2s2r7v8q)
- [Discord](https://discord.gg/sAaGuyW3D2)
- [Docs](https://docs.daodao.zone)
- [Manually Instantiating a DAO](https://github.com/DA0-DA0/dao-contracts/wiki/Instantiating-a-DAO)
- [Twitter](https://github.com/DA0-DA0)
- [What is a DAO?](https://docs.daodao.zone/docs/introduction/what-is-dao)

## Developers

Information about our development workflow and how to contribute can
be found in [CONTRIBUTING.md](./CONTRIBUTING.md).

## Testing

### Unit tests

Run `cargo test`, or `just test` from the project root to run the unit
tests.

### Integration tests

Run `just bootstrap-dev` to spin up a local environment and `just
integration-test-dev` to run tests against it.

See [ci/integration-tests/README.md](ci/integration_tests/README.md)
for more information.

## Disclaimer

DAO DAO TOOLING IS PROVIDED “AS IS”, AT YOUR OWN RISK, AND WITHOUT
WARRANTIES OF ANY KIND. No developer or entity involved in creating
the DAO DAO UI or smart contracts will be liable for any claims or
damages whatsoever associated with your use, inability to use, or your
interaction with other users of DAO DAO tooling, including any direct,
indirect, incidental, special, exemplary, punitive or consequential
damages, or loss of profits, cryptocurrencies, tokens, or anything
else of value.
