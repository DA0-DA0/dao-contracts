# DAO DAO Contracts

[![codecov](https://codecov.io/gh/DA0-DA0/dao-contracts/branch/main/graph/badge.svg?token=SCKOIPYZPV)](https://codecov.io/gh/DA0-DA0/dao-contracts)

This is a collection for smart contracts for building composable,
modular, and upgradable DAOs.

For an overview of our contract design, see [our
wiki](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design).

| Audited contracts                                                        | Description                                                |
| :----------------------------------------------------------------------- | :--------------------------------------------------------- |
| [cw-core](contracts/cw-core)                                             | The core module for a DAO DAO DAO.                         |
| [cw-proposal-single](contracts/cw-proposal-single)                       | A proposal module for single choice (yes / no) voting.     |
| [cw20-staked-balance-voting](contracts/cw20-staked-balance-voting)       | A voting power module for staked governance tokens.        |
| [cw4-voting](contracts/cw4-voting)                                       | A voting power module for multisig-style voting.           |
| [cw20-stake](contracts/cw20-stake)                                       | A contract for staking cw20 tokens.                        |
| [stake-external-rewards](contracts/cw20-stake-external-rewards)          | A contract for providing external stakinig rewards.        |
| [cw20-stake-reward-distributor](contracts/cw20-stake-external-rewards)   | A contract for distributing rewards via stake-cw20.        |


| Unaudited contracts                                                      | Description                                                |
| :----------------------------------------------------------------------- | :--------------------------------------------------------- |
| [cw-named-groups](contracts/cw-named-groups)                             | A contract for managing named groups of addresses.         |
| [cw-proposal-multiple](contracts/cw-proposal-multiple)                   | A proposal module for multiple choice proposals.           |

Audited contracts have completed audits by
[securityDAO](https://github.com/securityDAO/audits/blob/7bb8e4910baaea89fddfc025591658f44adbc27c/cosmwasm/dao-contracts/v0.3%20DAO%20DAO%20audit.pdf)
and [Oak
Security](https://github.com/oak-security/audit-reports/blob/2377ba8cfcfd505283c789d706311b06771d6db4/DAO%20DAO/2022-06-22%20Audit%20Report%20-%20DAO%20DAO%20v1.0.pdf).

## Developers

Information about our development workflow and how to contribute can
be found in [CONTRIBUTING.md](./CONTRIBUTING.md).

## Testing

### Unit tests

Run `cargo t` or `cargo unit-test` from the project root to run the unit tests.

### Integration tests

* `./scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg`
* `cd ci/integration_tests`
* `CONTRACT_DIR="../../artifacts" GAS_OUT_DIR="gas_reports" CONFIG="configs/local.yaml" cargo t`

See [ci/integration_tests/README.md](ci/integration_tests/README.md) for more information.

## Disclaimer

DAO DAO TOOLING IS PROVIDED “AS IS”, AT YOUR OWN RISK, AND WITHOUT
WARRANTIES OF ANY KIND. No developer or entity involved in creating
the DAO DAO UI or smart contracts will be liable for any claims or
damages whatsoever associated with your use, inability to use, or your
interaction with other users of DAO DAO tooling, including any direct,
indirect, incidental, special, exemplary, punitive or consequential
damages, or loss of profits, cryptocurrencies, tokens, or anything
else of value.
