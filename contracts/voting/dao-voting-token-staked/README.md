# `dao_voting_token_staked`

Simple native or Token Factory based token voting / staking contract which assumes the native denom provided is not used for staking for securing the network e.g. IBC denoms or secondary tokens (ION). Staked balances may be queried at an arbitrary height. This contract implements the interface needed to be a DAO DAO [voting module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

### Token Factory support
`dao_voting_token_staked` leverages the `cw_tokenfactory_issuer` contract for tokenfactory functionality. When instantiated, `dao_voting_token_staked` creates a new `cw_tokenfactory_issuer` contract to manage the new Token, with the DAO as admin and owner (these can be renounced or updated by vote of the DAO).

## Instantiation
When instantiating a new `dao_voting_token_staked` contract there are two required fields:
- `token_info`: you have the option to leverage an `existing` token or creating a `new` one.

There are a few optional fields:
- `unstaking_duration`: can be set to `height` or `time` (in seconds), this is the amount of time that must elapse before a user can claim fully unstaked tokens. If not set, they are instantly claimable.
- `active_theshold`: the amount of tokens that must be staked for the DAO to be active. This may be either an `absolute_count` or a `percentage`.

### Create a New Token
- `token_issuer_code_id`: must be set to a valid Code ID for the `cw_tokenfactory_issuer` contract.
Creating a token has a few additional optional fields:
- `metadata`: information about the token. See [Cosmos SDK Coin metadata documentation](https://docs.cosmos.network/main/architecture/adr-024-coin-metadata) for more info on coin metadata.
- `initial_dao_balance`: the initial balance created for the DAO.

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
Example insantiation mesggage:

``` json
{
    "token_info": {
      "new": {
        "subdenom": "uion",
      }
}
```
