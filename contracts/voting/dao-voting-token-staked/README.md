# `dao_voting_token_staked`

Simple native or Token Factory based token voting / staking contract which assumes the native denom provided is not used for staking for securing the network e.g. IBC denoms or secondary tokens (ION). Staked balances may be queried at an arbitrary height. This contract implements the interface needed to be a DAO DAO [voting module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

### Token Factory support
`dao_voting_token_staked` leverages the `cw_tokenfactory_issuer` contract for tokenfactory functionality. When instantiated, `dao_voting_token_staked` creates a new `cw_tokenfactory_issuer` contract to manage the new Token, with the DAO as admin and owner (these can be renounced or updated by vote of the DAO).

The `cw_tokenfactory_issuer` contract supports many features, see the [cw_tokenfactory_issuer contract README](../../external/cw-tokenfactory-issuer/README.md) for more information.

## Instantiation
When instantiating a new `dao_voting_token_staked` contract there are two required fields:
- `token_info`: you have the option to leverage an `existing` native token or creating a `new` one using the Token Factory module.

There are a few optional fields:
- `unstaking_duration`: can be set to `height` or `time` (in seconds), this is the amount of time that must elapse before a user can claim fully unstaked tokens. If not set, they are instantly claimable.
- `active_theshold`: the amount of tokens that must be staked for the DAO to be active. This may be either an `absolute_count` or a `percentage`.

### Create a New Token
- `token_issuer_code_id`: must be set to a valid Code ID for the `cw_tokenfactory_issuer` contract.
- `initial_balances`: the initial distribution of the new token, there must be at least 1 account with a balance so as the DAO is not locked.

Creating a token has a few additional optional fields:
- `metadata`: information about the token. See [Cosmos SDK Coin metadata documentation](https://docs.cosmos.network/main/architecture/adr-024-coin-metadata) for more info on coin metadata.
- `initial_dao_balance`: the initial balance created for the DAO treasury. 

Example insantiation mesggage:
``` json
{
  "token_info": {
    "new": {
      "token_issuer_code_id": <cw_tokenfactory_issuer_code_id>,
      "subdenom": "meow",
      "metadata": {
        "description": "Meow!",
        "additional_denom_units": [
          {
            "denom": "roar",
            "exponent": 6,
            "aliases": []
          }
        ],
        "display": "meow",
        "name": "Cat Token",
        "symbol": "MEOW"
      },
      "initial_balances": [
        {
          "amount": "100000000",
          "address": "<address>"
        }
      ],
      "initial_dao_balance": "100000000000"
    }
  },
  "unstaking_duration": {
    "time": 100000
  },
  "active_threshold": {
    "percentage": {
      "percent": "0.1"
    }
  }
}
```

### Use Existing Native Token
`dao-voting-token-staked` can also be used with existing native tokens. They could be in the form of a native denom like `ion`, an IBC token, or a Token Factory token.

Example insantiation mesggage:

``` json
{
    "token_info": {
      "existing": {
        "denom": "uion",
      }
    }
}
```

NOTE: if using an existing Token Factory token, double check the Token Factory admin and consider changing the Token Factory to be the DAO after the DAO is created.

### Use a factory
Occassionally, more customization is needed. Maybe you want to have an Augmented Bonding Curve contract or LP pool that requires additional setup? It's possible with factory contracts!

The `factory` pattern takes a single `WasmMsg::Execute` message that calls into a custom factory contract.

**NOTE:** when using the factory pattern, it is important to only use a trusted factory contract, as all validation happens in the factory contract.

Those implementing custom factory contracts MUST handle any validation that is to happen, and the custom `WasmMsg::Execute` message MUST include `TokenFactoryCallback` data respectively.

The [dao-test-custom-factory contract](../test/dao-test-custom-factory) provides an example of how this can be done and is used for tests. It is NOT production ready, but meant to serve as an example for building factory contracts.
