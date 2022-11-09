# DAO DAO Contracts

[![codecov](https://codecov.io/gh/DA0-DA0/dao-contracts/branch/main/graph/badge.svg?token=SCKOIPYZPV)](https://codecov.io/gh/DA0-DA0/dao-contracts)

This is a collection of smart contracts for building composable,
modular, and upgradable DAOs.

For an overview of our contract design, see [our
wiki](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design).

| Audited contracts (tag: v1.0.0)                                                | Description                                            |
|:-------------------------------------------------------------------------------|:-------------------------------------------------------|
| [cwd-core](contracts/cwd-core)                                                 | The core module for a DAO DAO DAO.                     |
| [cwd-proposal-single](contracts/proposal/cwd-proposal-single)                  | A proposal module for single choice (yes / no) voting. |
| [cwd-voting-cw20-staked](contracts/voting/cwd-voting-cw20-staked)              | A voting power module for staked governance tokens.    |
| [cwd-voting-cw4](contracts/voting/cwd-voting-cw4)                              | A voting power module for multisig-style voting.       |
| [cw20-stake](contracts/staking/cw20-stake)                                     | A contract for staking cw20 tokens.                    |
| [cw20-stake-external-rewards](contracts/staking/cw20-stake-external-rewards)   | A contract for providing external stakinig rewards.    |
| [cw20-stake-reward-distributor](contracts/staking/cw20-stake-external-rewards) | A contract for distributing rewards via stake-cw20.    |

| Unaudited contracts                                                                   | Description                                                                            |
|:--------------------------------------------------------------------------------------|:---------------------------------------------------------------------------------------|
| [cwd-proposal-multiple](contracts/proposal/cwd-proposal-multiple)                     | A proposal module for multiple choice proposals.                                       |
| [cwd-voting-cw721-staked](contracts/voting/cwd-voting-cw721-staked)                   | A voting module based on staked NFTs                                                   |
| [cwd-pre-propose-single](contracts/pre-propose/cwd-pre-propose-single)                | A pre-propose module for single choice proposals.                                      |
| [cwd-voting-native-staked](contracts/proposal/cwd-voting-native-staked)               | A voting power based on staked native tokens not used to secure the chain e.g. ION.    |
| [cwd-voting-staking-denom-staked](contracts/proposal/cwd-voting-staking-denom-staked) | A voting power module based on staked native tokens used to secure the chain e.g. JUNO |
| [cwd-pre-propose-multiple](contracts/pre-propose/cwd-pre-propose-multiple)            | A pre-propose module for multiple choice proposals.                                    |
|                                                                                       |                                                                                        |

Audited contracts have completed audits by
[securityDAO](https://github.com/securityDAO/audits/blob/7bb8e4910baaea89fddfc025591658f44adbc27c/cosmwasm/dao-contracts/v0.3%20DAO%20DAO%20audit.pdf)
and [Oak
Security](https://github.com/oak-security/audit-reports/blob/2377ba8cfcfd505283c789d706311b06771d6db4/DAO%20DAO/2022-06-22%20Audit%20Report%20-%20DAO%20DAO%20v1.0.pdf)
on the `v1.0.0` tag. An audit for the v2 contracts is forthcoming.

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
