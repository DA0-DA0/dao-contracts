# DAO DAO Contracts

| Contract                                    | Description                                                       |
| :------------------------------------------ | :---------------------------------------------------------------- |
| [cw-distribution](contract/cw-distribution) | A contract for distributing staking rewards.                      |
| [cw3-dao](contracts/cw3-dao)                | A DAO with voting power based on staked governance tokens.        |
| [cw3-multisig](contract/cw3-multisig)       | A multisig contract                                               |
| [cw4-registry](contract/cw4-registry)       | A contract for indexing multisig group members                    |
| [stake-cw20](contract/stake-cw20)           | A cw20 staking contract                                           |
| [stake-cw20-gov](contract/stake-cw20-gov)   | A cw20 staking contract, with vote delegation (used by `cw3-dao`) |

NOTE: _These contracts have yet to be audited. Please see the [disclaimer](#Disclaimer)._

## Contributing

Interested in contributing to DAO DAO? Check out [CONTRIBUTING.md](./CONTRIBUTING.md).

## Deploying in a development environment

Build and deploy the contracts to a local chain running in Docker with:

```sh
bash scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg
```

> Note: This Wasm account is from the [default account](default-account.txt), which you can use for testing (DO NOT store any real funds with this account). You can pass in any wasm account address you want to use.

This will run a chain locally in a docker container, then build and deploy the contracts to that chain.

The script will output something like:

```sh
NEXT_PUBLIC_DAO_TOKEN_CODE_ID=1
NEXT_PUBLIC_DAO_CONTRACT_CODE_ID=2
NEXT_PUBLIC_MULTISIG_CODE_ID=3
NEXT_PUBLIC_C4_GROUP_CODE_ID=4
NEXT_PUBLIC_STAKE_CW20_CODE_ID=5
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

## Disclaimer

DAO DAO TOOLING IS PROVIDED “AS IS”, AT YOUR OWN RISK, AND WITHOUT WARRANTIES OF ANY KIND. No developer or entity involved in creating the DAO DAO UI or smart contract will be liable for any claims or damages whatsoever associated with your use, inability to use, or your interaction with other users of DAO DAO tooling, including any direct, indirect, incidental, special, exemplary, punitive or consequential damages, or loss of profits, cryptocurrencies, tokens, or anything else of value.
