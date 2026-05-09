# cw-abc

Implements an [Augmented Bonding Curve](https://medium.com/commonsstack/deep-dive-augmented-bonding-curves-b5ca4fad4436).

Forked from and heavily inspired by the work on [cw20-bonding](https://github.com/cosmwasm/cw-tokens/tree/main/contracts/cw20-bonding). This contract uses native and token factory tokens instead.

NOTE: this contract is NOT AUDITED and experimental. NOT RECOMMENDED FOR PRODUCTION USE. Use at your own risk.

## What are Augmented Bonding Curves?
Before we get to the *Augmented* part, we must first describe bonding curves themselves.

### Token Bonding Curves

"A token bonding curve (TBC) is a mathematical curve that defines a relationship between price and token supply." ~[Aavegotchi Wiki](https://wiki.aavegotchi.com/en/curve)

Each bonding curve has a pricing function, also known as the price curve (or `curve_fn` in our implementation). The `curve_fn` is used to determine the price of the asset.

With bonding curves, we will always know what the price of an asset will be based on supply! More on benefits later.

This contract implements two methods:
- `Buy {}` is called with sending along some reserve currency (such as $USDC, or whatever the bonding curve is backed by). The reserve currency is stored by the bonding curve contract, and new tokens are minted and sent to the user.
- `Sell {}` is called along with sending some supply currency (the token minted by the bonding curve). The supply tokens are burned, and reserve currency is returned.

It is possible to use this contact as a basic bonding curve, without any of the augmented features.

#### Math

Given a price curve `f(x)` = price of the `x`th token, we want to figure out how to buy into and sell from the bonding curve. In fact we can look at the total supply issued. let `F(x)` be the integral of `f(x)`. We have issued `x` tokens for `F(x)` sent to the contract. Or, in reverse, if we send `x` tokens to the contract, it will mint `F^-1(x)` tokens.

From this we can create some formulas. Assume we currently have issued `S` tokens in exchange for `N = F(S)` input tokens. If someone sends us `x` tokens, how much will we issue?

`F^-1(N+x) - F^-1(N)` = `F^-1(N+x) - S`

And if we sell `x` tokens, how much we will get out:

`F(S) - F(S-x)` = `N - F(S-x)`

Just one calculation each side. To be safe, make sure to round down and always check against `F(S)` when using `F^-1(S)` to estimate how much should be issued. This will also safely give us how many tokens to return.

There is built in support for safely [raising i128 to an integer power](https://doc.rust-lang.org/std/primitive.i128.html#method.checked_pow). There is also a crate to [provide nth-root of for all integers](https://docs.rs/num-integer/0.1.43/num_integer/trait.Roots.html). With these two, we can handle most math except for logs/exponents.

Compare this to [writing it all in solidity](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/7b7ff729b82ea73ea168e495d9c94cb901ae95ce/contracts/math/Power.sol)

Examples:

Price Constant: `f(x) = k` and `F(x) = kx` and `F^-1(x) = x/k`

Price Linear: `f(x) = kx` and `F(x) = kx^2/2` and `F^-1(x) = (2x/k)^(0.5)`

Price Square Root: `f(x) = x^0.5` and `F(x) = x^1.5/1.5` and `F^-1(x) = (1.5*x)^(2/3)`

[You can read more about bonding curve math here](https://yos.io/2018/11/10/bonding-curves/).

#### Benefits

There are a number of benefits to bonding curves:
- There is enough liquidity to back the entire supply without having to list tokens on DEXs
- Easier to wind down projects (there is no going to zero)
- Transparent pricing: looking at the curve will tell you a lot about what kind of project it is.

### Augmented Bonding Curves

Augmented Bonding Curves are nothing new, some articles that inspired this implementation:
- https://medium.com/commonsstack/deep-dive-augmented-bonding-curves-b5ca4fad4436
- https://tokeneconomy.co/token-bonding-curves-in-practice-3eb904720cb8

At a high level, augmented bonding curves extend bonding curves with new functionality:
- Entry and exit fees
- Different phases representing the life cycles of projects

## Features

Example Instantiation message:

``` json
{
    "fees_recipient": "address that receives fees",
    "token_issuer_code_id": 0,
    "supply": {
        "subdenom": "utokenname",
        "metadata": {
            "name": "tokenname",
            "description": "Token description.",
            "symbol": "TOKEN",
            "display": "Token",
        },
        "decimals": 6,
        "max_supply": "100000000000000"
    },
    "reserve": {
        "denom": "ujuno",
        "decimals": 6,
    },
    "curve_type": {
        "linear": {
            "slope": "2",
            "scale": 1
        }
    },
    "phase_config": {
        "hatch": {
            "contribution_limits": {
                "min": "10000000",
                "max": "100000000000"
            },
            "initial_raise": {
                "min": "10000000",
                "max": "100000000000"
            },
            "entry_fee": "0.25"
        },
        "open": {
            "exit_fee": "0.01",
            "entry_fee": "0.01"
        },
        "closed": {}
    },
    "hatcher_allowlist": [
        {
            "addr": "dao_address",
            "config": {
                "config_type": { "dao": { "priority": 1 } },
                "contribution_limits_override": {
                    "min": "100000000",
                    "max": "99999999999999"
                }
            }
        },
        {
            "addr": "address",
            "config": {
                "config_type": { "address": {} }
            }
        }
    ],
}
```

- `fees_recipient`: the address that will receive fees (usually a DAO).
- `token_issuer_code_id`: the CosmWasm code ID for a `cw-tokenfactory_issuer` contract.
- `supply`: info about the token that will be minted by the curve. This is the token that is created by the bonding curve.
- `reserve`: this is the token that is used to mint the supply token.
- `curve_type`: information about the pricing curve.
- `phase_config`: configuration for the different phase of the augmented bonding curve.
- `hatcher_allowlist`: the list of address allowed to participate in a hatch.

## Trust assumptions

This contract has a privileged `owner` address (set at instantiate, transferable via `UpdateOwnership`) that can:

- Pause the contract (circuit breaker).
- Withdraw the funding pool (`Withdraw`).
- Update the curve type — but **only in the Closed phase**, and only when the new curve produces a reserve at the existing supply within 1% of the recorded reserve (audit fix C-1, see `audits/`).
- Update phase config (Hatch and Open variants).
- Update the maximum supply (`UpdateMaxSupply`).
- Update the hatcher allowlist (`UpdateHatchAllowlist`).
- Close the curve (`Close`).
- Update the funding-pool forwarding address.

**Recommended deployment**: ownership set to a DAO core contract. EOA / single-multisig ownership is technically supported but not recommended for production deployments; the owner has unilateral access to the funding pool and substantial latitude to adjust phase config.

`AbortHatch {}` is permissionless — any address can call it after `hatch_deadline` has passed if the curve has not reached `initial_raise.min`. This transitions the contract to Closed so hatchers can recover their reserve.

`UpdateCurve` is gated on Closed phase plus a continuity check that prevents replacement curves from breaking the (reserve, supply) invariant beyond a small tolerance. This closes the rug-via-curve-swap surface identified in the 2026-05-09 security review.

## Vesting (added 2026-05-09)

Hatcher tokens are subject to a configurable vesting schedule once the curve transitions Hatch → Open. See `CommonsPhaseConfig.vesting`:

- `VestingSchedule::None` — no vesting; hatchers can sell immediately at Open.
- `VestingSchedule::Cliff { duration_seconds }` — 0% available until `duration_seconds` after Open transition, 100% after.
- `VestingSchedule::Linear { duration_seconds }` — linear ramp from 0% at Open transition to 100% at `duration_seconds`.

Tokens minted during the Open phase by addresses that did **not** participate in the hatch are not subject to vesting.

## Future Work

- [ ] Implement an expanded set of pricing curves to choose from
- [ ] Hatch-failure refund path: in a future `Refunding` sub-state, hatchers receive a pro-rata share of `reserve + funding` (currently `AbortHatch` only refunds the reserve side).

