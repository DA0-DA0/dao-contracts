# DAO DAO Contracts

**NOT PRODUCTION READY**

| Contract                                        | Description                                            |
|:------------------------------------------------|:-------------------------------------------------------|
| [cw3-dao](contracts/cw3-dao)                    | A governance token based DAO.                          |
| [cw20-gov](contract/cw20-gov)                   | A cw20 token for use with cw3-dao                      |
| [cw3-multisig](contract/cw3-multisig) | A multisig contract                                                       |

## Deploying in a development environment

Deploy the contract to a local chain with:

``` sh
bash scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg
```

> Note: This Wasm account is from the [default account](default-account.txt), which you can use for testing (DO NOT store any real funds with this account). You can pass in any wasm account address you want to use.

This will run a chain locally in a docker container, then build and deploy the contracts to that chain.

The script will output something like:

``` sh
NEXT_PUBLIC_DAO_TOKEN_CODE_ID=1
NEXT_PUBLIC_DAO_CONTRACT_CODE_ID=2
NEXT_PUBLIC_MULTISIG_CODE_ID=3
NEXT_PUBLIC_C4_GROUP_CODE_ID=4
NEXT_PUBLIC_DAO_TOKEN_ADDRESS=juno14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9skjuwg8
NEXT_PUBLIC_DAO_CONTRACT_ADDRESS=juno1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq68ev2p
```

Add these to your frontend environment variables (`.env.local`).

You can then interact with the contract addresses.

Note, to send commands to the docker container:

``` sh
docker exec -i cosmwasm  junod status
```

Some commands require a password which defaults to `xxxxxxxxx`. You can use them like so:

``` sh
echo xxxxxxxxx | docker exec -i cosmwasm  junod keys show validator -a
```
