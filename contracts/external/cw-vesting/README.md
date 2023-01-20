# CW Payroll

This contract enables the creation of native && cw20 token streams, which allows a payment to be vested continuously over time. 

Key features include: 
- Optional contract owner, with ability to cancel payments
- Support for native and cw20 tokens
- Allows for automated distribution via external parties
- For payments in a chain governance token, the ability to stake and claim staking rewards
- Complex configuration for vesting schedules powered by [wynd-utils](https://github.com/cosmorama/wynddao/tree/main/packages/utils)

## Instantiation

To instantiate a new instance of this contract you may specify a contract owner, as well as payment parameters.

`cw-payroll-factory` can be used if wish to instantiate many `cw-vesting` contracts and query them.

## Creating a CW20 Vesting
A cw20 vesting payment can be created using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the vesting contract.

## Distribute payments
Vesting payments can be claimed continously at any point after the start time by triggering a Distribute message.

Anyone can call the distribute message, allowing for agents such as [Croncat](https://cron.cat/) to automatically trigger payouts.

## Staking native tokens
This contract allows for underlying native tokens to be staked.

### Limitations
While this contract allows for delegating native tokens, it does not allow for voting. As such, be sure to pick validators you delegate to wisely when using this contract.

## Cancelation
This vesting contract supports optional cancelation. This is only possible if an `owner` is set upon contract instantiation, otherwise the vesting contract cannot be altered by either party.

For example, if an employee has to leave a company for whatever reason, the company can vote to have the employee salary canceled.

When a contract is canceled, funds that have vested up until that moment are paid out to the `recipient` and the rest are refunded to the contract `owner`.

If funds are delegated when a contract is canceled, the delegated funds are immediately unbonded. After newly undelegated funds have finished the unbonding period, they can be withdraw by calling the `distribute` method to resolve.
