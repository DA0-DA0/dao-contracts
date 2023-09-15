# `cw-tokenfactory-issuer`

Forked from [osmosis-labs/cw-tokenfactory-issuer](https://github.com/osmosis-labs/cw-tokenfactory-issuer).

This repo contains a set of contracts that when used in conjunction with the x/tokenfactory module in Osmosis, Juno, and many other chains will enable a centrally issued stablecoin with many features:
- Creating a new Token Factory token or using an existing one
- Granting and revoking allowances for the minting and burning of tokens
- Updating token metadata
- Updating the contract owner or Token Factory admin
- And more! (see [Advanced Features](#advanced-features))

It is intended to work on multiple chains supporting Token Factory, and has been tested on Juno Network and Osmosis.

The contract has an owner (which can be removed or updated via `ExecuteMsg::UpdateOwnership {}`), but it can delegate capabilities to other acccounts. For example, the owner of a contract can delegate minting allowance of 1000 tokens to a new address. 

Ownership functionality for this contract is implemented using the `cw-ownable` library.

The `cw_tokenfactory_issuer` contract is also the admin of newly created Token Factory denoms. For minting and burning, users then interact with the contract using its own ExecuteMsgs which trigger the contract's access control logic, and the contract then dispatches tokenfactory sdk.Msgs from its own contract account.

## Instantiation

When instantiating `cw-tokenfactory-issuer`, you can either create a `new` or an `existing`.

### Creating a new Token Factory token

To create a new Token Factory token, simply instantiate the contract with a `subdenom`, this will create a new contract as well as a token with a denom formatted as `factory/{contract_address}/{subdenom}`.

Example instantiate message:

```json
{
  "new_token": {
    "subdenom": "test"
  }
}
```

All other updates can be preformed afterwards via this contract's `ExecuteMsg` enum. See `src/msg.rs` for available methods.

### Using an Existing Token

You can also instantiate this contract with an existing token, however most features will not be available until the previous Token Factory admin transfers admin rights to the instantiated contract and optionally calls `ExecuteMsg::SetBeforeSendHook {}` to enable dependent features.

Example instantiate message:

```json
{
  "existing_token": {
    "denom": "factory/{contract_address}/{subdenom}"
  }
}
```

## Renouncing Token Factory Admin
Some DAOs or protocols after the initial setup phase may wish to render their tokens immutable, permanently disabling features of this contract.

To do so, they must execute a `ExcuteMessage::UpdateTokenFactoryAdmin {}` method, setting the Admin to a null address or the bank module for your respective chain.

For example, on Juno this could be:

``` json
{
  "update_token_factory_admin": {
    "new_admin": "juno1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"
  }
}
```

The Token Factory standard requires a Token Factory admin per token, by setting to a null address the Token is rendered immutable and the `cw-tokenfactory-issuer` will be unable to make future updates. This is secure as the cryptography that underlies the chain enforces that even with the largest super computers in the world it would take an astonomically large amount of time to compute the private key for this address.

### Advanced Features

This contract supports a number of advanced features which DAOs or token issuers may wist to leverage:
- Freezing and unfreezing transfers, with an allowlist to allow specified addresses to allow transfer to or from
- Denylist to prevent certain addresses from transferring
- Force transfering tokens via the contract owner

**By default, these features are disabled**, and must be explictly enabled by the contract owner (for example via a DAO governance prop).

Moreover, for these features to work, your chain must support the `MsgBeforeSendHook` bank module hook. This is not yet available on every chain using Token Factory, and so denylisting and freezing features are not available if `MsgBeforeSendHook` is not supported.

On chains where `MsgBeforeSendHook` is supported, DAOs or issuers wishing to leverage these features must set the before send hook with `ExecuteMsg::SetBeforeSendHook {}`.

This method takes a `cosmwasm_address`, which is the address of a contract implement a `SudoMsg::BlockBeforeSend` entrypoint. Normally this will be the address of the `cw_tokenfactory_issuer` contract itself, but it is possible to specify a custom contract. This contract contains a `SudoMsg::BlockBeforeSend` hook that allows for the denylisting of specific accounts as well as the freezing of all transfers if necessary. 

Example message to set before send hook:
``` json
{
  "set_before_send_hook": {
    "cosmwasm_address": "<address of your cw_tokenfactory_issuer contract>"
  }
}
```

DAOs or issuers wishing to leverage these features on chains without support can call `ExecuteMsg::SetBeforeSendHook {}` when support is added.

If a DAO or issuer wishes to disable and removed before send hook related functionality, they simply need to call `ExecuteMsg::SetBeforeSendHook {}` with an empty string for the `cosmwasm_address` like so:
``` json
{
  "set_before_send_hook": {
    "cosmwasm_address": ""
  }
}
```
