# cw-abc

Implments an augmented bonding curve.

Forked from and heavily inspired by the work on [cw20-bonding](https://github.com/cosmwasm/cw-tokens/tree/main/contracts/cw20-bonding). This contract uses native and token factory tokens instead.

NOTE: this contract is unaudited and experimental. NOT RECOMMENDED FOR PRODUCTION USE.

## Overview
Before we get to the *Augmented* part, we must first describe bonding curves themselves.

### Token Bonding Curves

"A token bonding curve (TBC) is a mathematical curve that defines a relationship between price and token supply." ~[Aavegotchi Wiki](https://wiki.aavegotchi.com/en/curve)

Each bonding curve has a pricing function, also known as the price curve (or `curve_fn` in our implementation). The `curve_fn` is used to determine the price of the asset.

With bonding curves, we will always know what the price of an asset will be based on supply! More on benefits later.

This contract implements two methods:
- `Buy {}` is called with sending along some reserve curency (such as $USDC, or whatever the bonding curve is backed by). The reserve currency is stored by the bonding curve contract, and new tokens are minted and sent to the user.
- `Sell {}` is called along with sending some supply currency (the token minted by the bonding curve). The supply tokens are burned, and reserve curency is returned.

It is possible to use this contact as a basic bonding curve, without any of the augmented features.

#### Benefits

There are a number of benefits to bonding curves:
- There is enough liquidity to back the entire supply
- Easier to wind down projects (there is no going to zero)
- Transparent pricing: looking at the curve will tell you a lot about what kind of project it is.
- DAO treasuries can be funded by entry and exit fees on the curve

## Extended Reading

Augmented Bonding Curves are nothing new, some articles that inspired this implementation:
- https://medium.com/commonsstack/deep-dive-augmented-bonding-curves-b5ca4fad4436
- https://tokeneconomy.co/token-bonding-curves-in-practice-3eb904720cb8

Read more about [bonding curve math here](https://yos.io/2018/11/10/bonding-curves/).

## Future Work

Taking inspiration from [this article](https://medium.com/commonsstack/deep-dive-augmented-bonding-curves-b5ca4fad4436) on augmented bonding curves:

- [ ] Optionally vest tokens during the hatch phase
- [ ] Update `cw-vesting` to allow for partcipating in DAOs?

### Math

Given a price curve `f(x)` = price of the `x`th token, we want to figure out
how to buy into and sell from the bonding curve. In fact we can look at
the total supply issued. let `F(x)` be the integral of `f(x)`. We have issued
`x` tokens for `F(x)` sent to the contract. Or, in reverse, if we send
`x` tokens to the contract, it will mint `F^-1(x)` tokens.

From this we can create some formulas. Assume we currently have issued `S`
tokens in exchange for `N = F(S)` input tokens. If someone sends us `x` tokens,
how much will we issue?

`F^-1(N+x) - F^-1(N)` = `F^-1(N+x) - S`

And if we sell `x` tokens, how much we will get out:

`F(S) - F(S-x)` = `N - F(S-x)`

Just one calculation each side. To be safe, make sure to round down and
always check against `F(S)` when using `F^-1(S)` to estimate how much
should be issued. This will also safely give us how many tokens to return.

There is built in support for safely [raising i128 to an integer power](https://doc.rust-lang.org/std/primitive.i128.html#method.checked_pow).
There is also a crate to [provide nth-root of for all integers](https://docs.rs/num-integer/0.1.43/num_integer/trait.Roots.html).
With these two, we can handle most math except for logs/exponents.

Compare this to [writing it all in solidity](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/7b7ff729b82ea73ea168e495d9c94cb901ae95ce/contracts/math/Power.sol)

Examples:

Price Constant: `f(x) = k` and `F(x) = kx` and `F^-1(x) = x/k`

Price Linear: `f(x) = kx` and `F(x) = kx^2/2` and `F^-1(x) = (2x/k)^(0.5)`

Price Square Root: `f(x) = x^0.5` and `F(x) = x^1.5/1.5` and `F^-1(x) = (1.5*x)^(2/3)`

We will only implement these curves to start with, and leave it to others to import this with more complex curves,
such as logarithms.
