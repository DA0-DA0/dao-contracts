# cw-vesting

[![cw-vesting on crates.io](https://img.shields.io/crates/v/cw-vesting.svg?logo=rust)](https://crates.io/crates/cw-vesting)
[![docs.rs](https://img.shields.io/docsrs/cw-vesting?logo=docsdotrs)](https://docs.rs/cw-vesting/latest/cw_vesting/)

This contract enables the creation of native && cw20 token streams, which allows a payment to be vested continuously over time.

Key features include:

- Optional contract owner, with ability to cancel payments
- Support for native and cw20 tokens
- Allows for automated distribution via external parties or tools like [CronCat](https://cron.cat/)
- For payments in a chain governance token, the ability to stake and claim staking rewards
- Complex configuration for vesting schedules powered by [wynd-utils](https://github.com/cosmorama/wynddao/tree/main/packages/utils)

## Instantiation

To instantiate a new instance of this contract you may specify a contract owner, as well as payment parameters.

`cw-payroll-factory` can be used if you wish to instantiate many `cw-vesting` contracts and query them.

### Parameters

The `owner` of a contract is optional. Contracts without owners are not able to be canceled. The owner can be set to the DAO making the payment or a neutral third party.

#### Vesting curves

This package uses the curve implementation from [wynd-utils](https://github.com/cosmorama/wynddao/tree/main/packages/utils).

It supports 2 types of [curves](https://docs.rs/wynd-utils/0.4.1/wynd_utils/enum.Curve.html) that represent the vesting schedule:

- Saturating Linear: vests at a linear rate with a start and stop time.
- Piecewise Linear: linearally interpolates between a set of `(time, vested)` points

##### Piecewise Linear

Piecsewise Curves can be used to create more complicated vesting
schedules. For example, let's say we have a schedule that vests 50%
over 1 month and the remaining 50% over 1 year. We can implement this
complex schedule with a Piecewise Linear curve.

Piecewise Linear curves take a `steps` parameter which is a list of
tuples `(timestamp, vested)`. It will then linearally interpolate
between those points to create the vesting curve. For example, given
the points `(0, 0), (2, 2), (4, 8)`, it would create a vesting curve
that looks like this:

```text
  8 +----------------------------------------------------------------------+
    |        +        +        +        +       +        +        +     ** |
  7 |-+                                                               ** +-|
    |                                                              ***     |
    |                                                            **        |
  6 |-+                                                        **        +-|
    |                                                       ***            |
  5 |-+                                                   **             +-|
    |                                                   **                 |
    |                                                ***                   |
  4 |-+                                            **                    +-|
    |                                            **                        |
  3 |-+                                       ***                        +-|
    |                                       **                             |
    |                                     **                               |
  2 |-+                              *****                               +-|
    |                         *******                                      |
  1 |-+               ********                                           +-|
    |          *******                                                     |
    |   *******       +        +        +       +        +        +        |
  0 +----------------------------------------------------------------------+
    0       0.5       1       1.5       2      2.5       3       3.5       4
```

As you can see, it travels through `(0, 0)` in a straight line to `(2,
2)`, then increases its slope and travels to `(4, 8)`.

A curve where 50% vests the first month starting January 1st 2023, and
the remaining 50% vests over the next year. For 100 Juno.

```json
{
    "piecewise_linear": [
        (1672531200, "0"),
        (1675209600, "50000000"),
        (1706745600, "100000000")
    ]
}
```

### Creating native token vesting

If vesting native tokens, you need to include the exact amount in native funds that you are vesting when you instantiate the contract.

### Creating a CW20 Vesting

A cw20 vesting payment can be funded using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the vesting contract.

## Distribute payments

Vesting payments can be claimed continuously at any point after the start time by triggering a Distribute message.

_Anyone_ can call the distribute message, allowing for agents such as [CronCat](https://cron.cat/) to automatically trigger payouts.

## Staking native tokens

This contract allows for underlying native tokens to be staked if they
match the staking token of the native chain (i.e. $JUNO on [Juno
Network](https://junonetwork.io)).

`Delegate`, `Undelegate`, `Redelegate`, and `SetWithdrawAddress` can
_only_ be called by the `recipient`. `WithdrawDelegatorReward` can be
called by anyone to allow for easy auto-compounding. Due to
limitations to our ability to inspect the SDK's state from CosmWasm,
only funds that may be redelegated immediately (w/o an unbonding
period) may be redelegated.

#### Limitations

While this contract allows for delegating native tokens, it does not
allow for voting. As such, be sure to pick validators you delegate to
wisely when using this contract.

## Cancellation

This vesting contract supports optional cancellation. For example, if
an employee has to leave a company for whatever reason, the company
can vote to have the employee salary canceled.

This is only possible if an `owner` address is set upon contract
instantiation, otherwise the vesting contract cannot be altered by
either party.

When a contract is cancelled, the following happens:

1. All liquid tokens (non-staked) in the vesting contract are used to
   settle any undistributed, vested funds owed to the receiver.
2. Any leftover liquid tokens are returned to the contract owner.
3. Calls to `Delegate` are `Redelegate` are disabled.
4. Calls to `Undelegate` are made permissionless (allowing anyone to
   undelegate the contract's staked tokens).
5. Any pending staking rewards are claimed by the owner, and future
   staking rewards are directed to the owner.

It is imagined that frontends will prompt visitors to execute
undelegations, or a bot will do so. The contract can not automatically
undelegate as that would allow a malicious vest receiver to stake to
many validators and make cancelation run out of gas, preventing the
contract from being cancelable and allowing them to continue to
receive funds.

## Stable coin support

This contract can be used with stable coins such as $USDC. It does not
yet support auto swapping to stables, however this feature can be
enabled with other contracts or tools like
[CronCat](https://cron.cat/).

DAOs always have an option of swapping to stables before creating a
vesting contract ensuring no price slippage. For example, a proposal
to pay someone 50% $USDC could contain three messages:

1. Swap 50% of grant tokens for $USDC
2. Instantiate a vesting contract for the $USDC
3. Instantiate a vesting contract for the native DAO token

## Attribution

Thank you to Wynd DAO for their previous work on
[cw20-vesting](https://github.com/cosmorama/wynddao/tree/main/contracts/cw20-vesting)
and their [curve
package](https://github.com/cosmorama/wynddao/tree/main/packages/utils)
which informed and inspired this contract's design.
