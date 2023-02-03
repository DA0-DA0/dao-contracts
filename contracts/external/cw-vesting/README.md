# cw-vesting

This contract enables the creation of native && cw20 token streams, which allows a payment to be vested continuously over time. 

Key features include: 
- Optional contract owner, with ability to cancel payments
- Support for native and cw20 tokens
- Allows for automated distribution via external parties or tools like [CronCat](https://cron.cat/)
- For payments in a chain governance token, the ability to stake and claim staking rewards
- Complex configuration for vesting schedules powered by [wynd-utils](https://github.com/cosmorama/wynddao/tree/main/packages/utils)

## Instantiation

To instantiate a new instance of this contract you may specify a contract owner, as well as payment parameters.

`cw-payroll-factory` can be used if wish to instantiate many `cw-vesting` contracts and query them.

### Parameters

The `owner` of a contract is optional. Contracts without owners are not able to be canceled. The owner can be set to the DAO making the payment or a neutral third party.

The `params` object holds details of the vesting payment parameters. These will be validated upon instantiation. `recipient` represents the account that will be receiving the funds. `amount` represents the amount of the funds to be vested (in micro units). `denom` represents the denomination of the token to be vested and supports either `native` or `cw20`. `vesting_schedule` takes a curve from [wynd-utils](https://github.com/cosmorama/wynddao/tree/main/packages/utils) and validates it to make sure it fully vests (more on this in the next section). `title` and `description` are both optional metadata useful for bookkeeping.

An example `cw-vesting` instantiation message could look like this:
```sh
junod tx wasm instantiate <code-id> '{
    "owner": "juno1ajdflkjklj23223...",
    "params": {
        "recipient": "juno1asdjflkasdjflkasdjflk...",
        "amount": "1000000000",
        "denom": {
            "native": "ujuno",
        },
        "vesting_schedule": {
            "saturating_linear": {
                "min_x": 5000000,
                "min_y": "1000000000",
                "max_x": 6000000,
                "max_y": "0"
            }
        },
        "title": "Optional title",
        "description": "Optional payment description."
    }
}' --from <your-key> --admin <optional-your-key> --amount 100000000ujuno
```

#### Vesting curves

This package uses the curve implementation from [wynd-utils](https://github.com/cosmorama/wynddao/tree/main/packages/utils).

It supports 2 types of [curves](https://docs.rs/wynd-utils/0.4.1/wynd_utils/enum.Curve.html) that represent the vesting schedule:
- Saturating Linear: vests at a linear rate with a start and stop time.
- Piecewise Linear: implements a more complex vesting schedule

Both use `x` and `y`, where `x` represents time in UNIX seconds and `y` represents the amount. To be valid, whatever curve you implement must be *decreasing* or instantiation will fail. 

##### Saturating Linear

Vests tokens at a linear rate with a start and stop time. Again, `x` represents time in UNIX seconds and `y` represents the amount. You can think of `min_x` as the start time, `min_y` as the full vesting amount, and `max_x` as the end time. Note, that `max_y` should always be "0" as the `vesting_schedule` needs to fully vest in order to be valid.

A 1 month vesting schedule of 100 $JUNO stating on January 1st 2023 and ending on February 1st 2023:
``` json
{
    "params": {
        ...
        "vesting_schedule": {
            "saturating_linear": {
                "min_x": 1672531200,
                "min_y": "100000000",
                "max_x": 1675209600,
                "max_y": "0"
            }
        }
    }
}
```

##### Piecewise Linear

Piecsewise Curves can be used to create more complicated vesting schedules. For example, let's say we have a schedule that vests 50% over 1 month and the remaining 50% over 1 year. We can implement this complex schedule with a Piecewise Linear curve.

Piecewise Linear curves take a `steps` parameter which is a list of tuples `(x, y)`. Again, `x` represents time in UNIX seconds and `y` represents the amount. Note the last step has `y` at zero as the `vesting_schedule` needs to fully vest in order to be valid. Time needs to go up, amount unvested needs to go down.

A curve where 50% vests the first month starting January 1st 2023, and the remaining 50% vests over the next year. For 100 Juno.

``` json
{
    "params": {
        ...
        "vesting_schedule": {
            "piecewise_linear": {
                "steps": [
                    (1672531200, "100000000"),
                    (1675209600, "50000000"),
                    (1706745600, "0")
                ]
            }
        }
    }
}
```

### Creating native token vesting
If vesting native tokens, you need to include the exact amount in native funds that you are vesting when you instantiate the contract.

### Creating a CW20 Vesting
A cw20 vesting payment can be created using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the vesting contract.

## Distribute payments
Vesting payments can be claimed continuously at any point after the start time by triggering a Distribute message.

*Anyone* can call the distribute message, allowing for agents such as [CronCat](https://cron.cat/) to automatically trigger payouts.

## Staking native tokens
This contract allows for underlying native tokens to be staked if they match the staking token of the native chain (i.e. $JUNO on [Juno Network](https://junonetwork.io)).

`Delegate`, `Undelegate`, `Redelegate`, and `SetWithdrawAddress` can *only* be called by the `recipient`. `WithdrawDelegatorReward` can be called by anyone to allow for easy auto-compounding.

#### Limitations
While this contract allows for delegating native tokens, it does not allow for voting. As such, be sure to pick validators you delegate to wisely when using this contract.

## Cancellation
This vesting contract supports optional cancellation. For example, if an employee has to leave a company for whatever reason, the company can vote to have the employee salary canceled.
 
This is only possible if an `owner` address is set upon contract instantiation, otherwise the vesting contract cannot be altered by either party.

When a contract is canceled, funds that have vested up until that moment are paid out to the `recipient` and the rest are refunded to the contract `owner`.

If funds are delegated when a contract is canceled, the delegated funds are immediately unbonded. After newly undelegated funds have finished the unbonding period, they can be withdrawn by calling the `distribute` method to resolve.

## Stable coin support

This contract can be used with stable coins such as $USDC. It does not yet support auto swapping to stables, however this feature can be enabled with other contracts or tools like [CronCat](https://cron.cat/).

DAOs always have an option of swapping to stables before creating a vesting contract ensuring no price slippage. For example, a proposal to pay someone 50% $USDC could contain three messages:
1. Swap 50% of grant tokens for $USDC
2. Instantiate a vesting contract for the $USDC
3. Instantiate a vesting contract for the native DAO token
