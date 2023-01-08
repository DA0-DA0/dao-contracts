# CW Payroll

This contract enables the creation of native && cw20 token streams, which allows a cw20 payment to be vested continuously over time. This contract is forked off of [cw20-streams](https://github.com/CosmWasm/cw-tokens/tree/main/contracts/cw20-streams) to enable additional features required by DAOs for payroll. Key items being: Admin, configurations for vesting, allowing external parties to distribute & more.

## Instantiation

To instantiate a new instance of this contract you must specify a contract owner.

```sh
junod tx wasm instantiate <code-id> '{"owner": "juno12xyz..."}'  --label "cw-payroll contract" --from <your-key> 
```

One `cw-payroll` contract can handle multiple vesting payments.

## Creating a Native Token Vesting Payment

Simply call the `create` contract method while sending the amount of native tokens needed.

## Creating a CW20 Vesting
A stream can be created using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the token streaming contract.

## Distribute payments
Streamed payments can be claimed continously at any point after the start time by triggering a Distribute message.

Anyone can call the distribute message, allowing for agens such as [Croncat](https://cron.cat/) to automatically trigger payouts.

## Staking native tokens
This contract allows for underlying native tokens to be staked.

### Limitations
While this contract allows for delegating native tokens, it does not allow for voting. As such, be sure to pick validators you delegate to wisely when using this contract.

## Cancelation
This vesting contract supports optional cancelation. This is only possible if an `owner` is set upon contract instantiation, otherwise the vesting contract cannot be altered by either party.

For example, if an employee has to leave a company for whatever reason, the company can vote to have the employee salary canceled.

When a contract is canceled, funds that have vested up until that moment are paid out to the `recipient` and the rest are refunded to the contract `owner`.

If funds are delegated when a contract is canceled, the delegated funds are immediately unbonded. After newly undelegated funds have finished the unbonding period, they can be withdraw by calling the `distribute_and_close` method to resolve.

