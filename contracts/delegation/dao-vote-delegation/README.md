# DAO Vote Delegation

[![dao-vote-delegation on
crates.io](https://img.shields.io/crates/v/dao-vote-delegation.svg?logo=rust)](https://crates.io/crates/dao-vote-delegation)
[![docs.rs](https://img.shields.io/docsrs/dao-vote-delegation?logo=docsdotrs)](https://docs.rs/dao-vote-delegation/latest/dao_vote_delegation/)

The `dao-vote-delegation` contract allows members of a DAO to delegate their
voting power to other members of the DAO who have registered as delegates. It
works in conjunction with voting and proposal modules, as well as the rewards
distributor, to offer a comprehensive delegation system for DAOs that supports
the following features:

- Fractional delegation of voting power on a per-proposal-module basis.
- Overridable delegate votes that can be overridden on a per-proposal basis by
  the delegator
- Delegate reward commission.

## Instantiation and Setup

This contract must be instantiated by the DAO.

### Hooks

After instantiating the contract, it is VITAL to set up the required hooks for
it to work. To compute delegate voting power correctly, this contract needs to
know about both voting power changes and votes cast on proposals as soon as they
happen.

This can be achieved using the `add_hook` method on voting/staking contracts
that support voting power changes, such as:

- `cw4-group`
- `dao-voting-cw721-staked`
- `dao-voting-token-staked`
- `cw20-stake`

For proposal modules, the corresponding hook is `add_vote_hook`:

- `dao-proposal-single`
- `dao-proposal-multiple`
- `dao-proposal-condorcet`

## Design Decisions

### Fractional Delegation via Percentages

In order to support fractional delegation, users assign a percentage of voting
power to each delegate. Percentages are used instead of choosing an absolute
amount of voting power (e.g. staked tokens) since voting power can change
independently of delegation. If an absolute amount were used, and a user who had
delegated all of their voting power to a few different delegates then unstaked
half of their tokens, there is no clear way to resolve what their new
delegations are. Using percentages instead allows voting power and delegation to
be decided independently.
