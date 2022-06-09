# DAO DAO Contracts
[![codecov](https://codecov.io/gh/DA0-DA0/dao-contracts/branch/main/graph/badge.svg?token=SCKOIPYZPV)](https://codecov.io/gh/DA0-DA0/dao-contracts)

| Audited contracts                                                  | Description                                            |
| :----------------------------------------------------------------- | :----------------------------------------------------- |
| [cw-core](contracts/cw-core)                                       | The core module for a DAO DAO DAO.                     |
| [cw-proposal-single](contracts/cw-proposal-single)                 | A proposal module for single choice (yes / no) voting. |
| [cw20-staked-balance-voting](contracts/cw20-staked-balance-voting) | A voting power module for staked governance tokens.    |
| [cw4-voting](contracts/cw4-voting)                                 | A voting power module for multisig-style voting.       |
| [stake-cw20](contracts/stake-cw20)                                 | A contract for staking cw20 tokens.                    |


| Unaudited contracts                                                    | Description                                              |
| :--------------------------------------------------------------------- | :------------------------------------------------------- |
| [cw-named-groups](contracts/cw-named-groups)                           | A contract for managing named groups of addresses.       |
| [cw-proposal-sudo](contracts/cw-proposal-sudo)                         | A proposal module that allows an admin to control a DAO. |
| [cw20-balance-voting](contracts/cw20-balance-voting)                   | TESTING ONLY - a voting module based on cw20 balances.   |
| [proposal-hooks-counter](contracts/proposal-hooks-counter)             | TESTING ONLY - a contract for testing proposal hooks.    |
| [stake-external-rewards](contracts/stake-cw20-external-rewards)        | A contract for providing external stakinig rewards.      |
| [stake-cw20-reward-distributor](contracts/stake-cw20-external-rewards) | A contract for distributing rewards via stake-cw20.      |

Audited contracts have completed [an
audit](https://github.com/securityDAO/audits/blob/7bb8e4910baaea89fddfc025591658f44adbc27c/cosmwasm/dao-contracts/v0.3%20DAO%20DAO%20audit.pdf)
by security DAO. A second audit is forthcoming.

## Contributing

Interested in contributing to DAO DAO? Check out [CONTRIBUTING.md](./CONTRIBUTING.md).

## Deploying in a development environment

_Note: this will deploy the legacy version of the contracts currently
running at [daodao.zone](https://daodao.zone)._

Build and deploy the contracts to a local chain running in Docker with:

```sh
bash scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg
```

> Note: This Wasm account is from the [default account](default-account.txt), which you can use for testing (DO NOT store any real funds with this account). You can pass in any wasm account address you want to use.

This will run a chain locally in a docker container, then build and deploy the contracts to that chain.

The script will output something like:

```sh
NEXT_PUBLIC_CW20_CODE_ID=1
NEXT_PUBLIC_CW4GROUP_CODE_ID=2
NEXT_PUBLIC_CWCORE_CODE_ID=5
NEXT_PUBLIC_CWPROPOSALSINGLE_CODE_ID=8
NEXT_PUBLIC_CW4VOTING_CODE_ID=4
NEXT_PUBLIC_CW20STAKEDBALANCEVOTING_CODE_ID=3
NEXT_PUBLIC_STAKECW20_CODE_ID=9
NEXT_PUBLIC_DAO_CONTRACT_ADDRESS=juno1wug8sewp6cedgkmrmvhl3lf3tulagm9hnvy8p0rppz9yjw0g4wtqwrw37d
```

You can then instantiate and interact with contracts.

Note, to send commands to the docker container:

```sh
docker exec -i cosmwasm junod status
```

Some commands require a password which defaults to `xxxxxxxxx`. You can use them like so:

```sh
echo xxxxxxxxx | docker exec -i cosmwasm  junod keys show validator -a
```

## Generating schema for all contracts
As we have a workflow to check schema differences on commit, to quickly run `cargo schema` against all contracts
simply run the following from the repo root:
```sh
./scripts/schema.sh
```

## Disclaimer

DAO DAO TOOLING IS PROVIDED “AS IS”, AT YOUR OWN RISK, AND WITHOUT WARRANTIES OF ANY KIND. No developer or entity involved in creating the DAO DAO UI or smart contracts will be liable for any claims or damages whatsoever associated with your use, inability to use, or your interaction with other users of DAO DAO tooling, including any direct, indirect, incidental, special, exemplary, punitive or consequential damages, or loss of profits, cryptocurrencies, tokens, or anything else of value.
