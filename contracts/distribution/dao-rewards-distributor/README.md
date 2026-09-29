# DAO Rewards Distributor

[![dao-rewards-distributor on
crates.io](https://img.shields.io/crates/v/dao-rewards-distributor.svg?logo=rust)](https://crates.io/crates/dao-rewards-distributor)
[![docs.rs](https://img.shields.io/docsrs/dao-rewards-distributor?logo=docsdotrs)](https://docs.rs/dao-rewards-distributor/latest/cw20_stake_external_rewards/)

The `dao-rewards-distributor` works in conjuction with DAO voting modules to
provide rewards streamed over time for DAO members. The contract supports both
native and CW20 Cosmos SDK tokens. Any voting power module that supports the
standard DAO voting module interface is supported for deriving staking reward
allocations, as long it also supports voting power change hooks. This includes,
but is not limited to:

- `dao-voting-cw4`: for membership or group based DAOs
- `dao-voting-cw20-staked`: for cw20 token based DAOs.
- `dao-voting-cw721-staked`: for NFT based DAOs.
- `dao-voting-token-staked`: for native and Token Factory token based DAOs.

## Instantiation and Setup

The contract is instantiated with a very minimal state. An optional `owner` can
be specified. If it is not, the owner is set to be the address instantiating the
contract.

### Hooks

After instantiating the contract, it is VITAL to set up the required hooks for
it to work. This is because to pay out rewards accurately, this contract needs
to know about staking or voting power changes in the DAO as soon as they happen.

This can be achieved using the `add_hook` method on contracts that support
voting power changes, such as:

- `cw4-group`
- `dao-voting-cw721-staked`
- `dao-voting-token-staked`
- `cw20-stake`

This contract rejects hook calls from any address that is not the
`hook_caller` of at least one distribution. Staking contracts treat a failed
hook call as non-fatal: the stake or unstake still succeeds, the hook stays
registered, and the failure is recorded as attributes on the staking
transaction, naming the receiver that failed (`action: stake_hook_failed`,
`addr: <receiver>`). This means a stale or premature hook registration can never
block staking, but it also means rewards are not being tracked while the hook is
misconfigured, so watch for that attribute and set up hooks and distributions
together, ideally in the same proposal.

### Creating a new distribution

Only the `owner` can create new distributions.

Creating a distribution requires the following configuration:

- `denom`, which can be a native token or CW20 contract
- `emission_rate`, which determines how the rewards are distributed. there are 3
  options:
  - `paused`: no rewards are distributed until the emission rate is updated
  - `immediate`: funded rewards are distributed immediately to those with
    voting power
  - `linear`: `amount` of the denom is distributed to all applicable addresses
    per `duration` of time, updating throughout based on changing voting power.
    `duration` may be declared in either time (seconds) or blocks. if
    `continuous` is true, it will backfill if there are funding gaps using
    current voting power. some example configurations may be:
    - `1000udenom` per `500 blocks`
    - `10udenom` per `24 hours`
    - `1udenom` per `1 second`
- `vp_contract` address, which will be used to determine the total and relative
  address voting power for allocating the rewards on a pro-rata basis
- `hook_caller` address, which will be authorized to call back into this
  contract with any voting power event changes. examples of such events may be:
  - user staking tokens
  - user unstaking tokens
  - user cw-721 state change event
  - cw-4 membership change event
- optional `withdraw_destination` address to be used when withdrawing (i.e.
  unfunding the remainder of a previously funded distribution). this may be a
  subDAO, for example. if not provided, the contract owner is used.

You can fund a distribution at any point after it's been created, or during
creation if it's for a native token. CW20 tokens must be funded after creation.
Simply including native funds in the create message will suffice. For any token,
you can always top up the funds later, which extends the distribution period.

### Funding a distribution

Anyone can fund a distribution once it's been created.

> **WARNING:** Do not transfer funds directly to the contract. You must use the
> `Fund` execution message in order for the contract to properly recognize and
> distribute the tokens. **Funds will be lost if you don't use the execution
> msg.**

There are a few different emission rates. Below describes the funding behavior
while different emission rates are active.

#### Linear

Linear emissions can be continuous or not.

When a linear emission is **continuous**, it will backfill rewards if there's a gap
between when it finishes distributing everything it's been funded with so far
and when it's funded next. This means that when another funding occurs after a
period of no more rewards being available, it will instantly distribute the
portion of the funds that corresponds with the time that passed in that gap. One
limitation is that it uses the current voting power to backfill.

When a linear emission is **not continuous**, and a gap in reward funding occurs, it
will simply restart the distribution the next time it receives funding. This may
be less intuitive, but it doesn't suffer from the voting power limitation that
the continuous mode does.

Upon funding, the start and end are computed based on the funds provided, the
configured emission rate, and whether or not it's set to the continuous mode. If
this is the first funding, or it's not continuous and we're restarting from the
current block, the start block is updated to the current block. The end block is
computed based on the start block and funding duration, calculated from the
emission rate and remaining funds, including any that already existed that have
not yet been distributed.

Linear emissions can be extended indefinitely by continuously funding them.

**Example:** if 100_000udenom were funded, and the configured emission rate is
1_000udenom per 100 blocks, we derive that there are 100_000/1_000 = 100 epochs
funded, each of which contain 100 blocks. We therefore funded 10_000 blocks of
rewards.

#### Immediate

When set to immediate, funding is immediately distributed based on the voting
power of the block funding occurs on.

You may fund an immediate distribution as many times as you'd like to distribute
funds instantly to the current members of the DAO.

#### Paused

When set to paused, no rewards will be distributed.

You may fund a paused distribution and accumulate rewards in the contract to be
distributed at a later date, since you can update the emission rate of a
distribution.

Maybe you want to accumulate rewards in a paused state for a month, and then
distribute them instantly at the end of the month to the DAO. Or maybe you want
to pause an active linear emission, which will hold the funds in the contract
and not distribute any more than have already been distributed.

### Updating emission rate and other distribution config

Only the `owner` can update a distribution's config.

Updating the emission rate preserves all previously distributed rewards and adds
it to a historical value (`historical_earned_puvp`), so updating does not
interfere with users who have not yet claimed their rewards.

You can also update the `vp_contract`, `hook_caller`, and
`withdraw_destination`.

> **WARNING:** You probably always want to update `vp_contract` and
> `hook_caller` together. Make sure you know what you're doing. And be sure to
> add/remove hooks on the old and new `hook_caller`s accordingly, ideally in the
> same proposal as the update.

### Withdrawing

Only the `owner` can withdraw from a distribution.

This is effectively the inverse of funding a distribution. If the current
distribution is inactive, meaning its emission rate is `paused`, `immediate`, or
`linear` with an expired distribution period (because the end block is in the
past), then there is nothing to withdraw.

When rewards are being distributed, withdrawing ends the distribution early,
setting the end block to the current one, and clawing back the undistributed
funds to the specified `withdraw_destination`. Pending funds that have already
been distributed, even if not yet claimed, will remain in the contract to be
claimed. Withdrawing only applies to unallocated funds.

### Claiming

You can claim funds from a distribution that you have pending rewards for.

### Conservative accrual for missed voting power change hooks

Rewards are accrued per address by multiplying that address's voting power by
the change in rewards earned per unit voting power (puvp) since that address's
last checkpoint. This is only correct if the address's voting power was
constant for that entire period, which voting power change hooks (see above)
are relied on to guarantee: any stake/unstake (or other voting power change)
is expected to checkpoint the address's reward state before its voting power
actually changes.

Some voting power/staking contracts allow a hook receiver to fail without
reverting the underlying stake change (so that one broken/misbehaving reward
distributor can't block staking). If a hook is ever missed this way, the
guarantee above breaks: an address's voting power can change without a
checkpoint, and naively using its current voting power for the whole
unaccounted-for period could over-credit it, potentially by more than the
distribution ever received.

To guard against this, accrual uses the *minimum* of:

- the address's voting power at the current block (the existing behavior),
  and
- the address's voting power at the block right after its last checkpoint for
  that distribution (voting power changes take effect on the following
  block).

In the healthy case (no missed hooks) these are always equal, since the
address's voting power hasn't changed since its last checkpoint, so this is
not a behavior change. After a single missed stake or unstake, the address is
conservatively *under*-credited for the affected period rather than
over-credited.

The baseline is never earlier than the block from which the distribution's
current `vp_contract` has been in use: its creation block (which also serves as
the baseline for an address that has never been checkpointed), or the block
`vp_contract` was last changed by `update`, since the new contract's earlier
voting power is unrelated to the distribution. Distributions and reward states
stored before this behavior was introduced don't have this information (the
distribution's creation height, or an address's last checkpoint height,
respectively); in that case, the original behavior (using only the current
voting power) applies, since there is no reference point to be conservative
against.

This mitigation only compares the two endpoints of a potentially-missed
window, so it cannot catch a missed change that moves an address's voting
power away from, and then back to, the same value within a single
unaccounted-for period (e.g. unstaking and then restaking the same amount
while disconnected). The per-distribution funds cap described below exists as
defense in depth against that residual case.

### Per-distribution funds cap

All distributions funded with the same denom share a single token balance on
this contract; `Claim` does not otherwise distinguish which distribution's
funds are being paid out. To ensure a bug in one distribution's reward
accounting (such as the residual over-credit risk described above) can never
pay out of a *different* distribution's funds, each distribution tracks its
own `claimable_funds`: the amount deposited via `Create`/`Fund`/`FundLatest`
that has not yet been claimed or clawed back via `Withdraw`. A `Claim` that
would reduce a distribution's `claimable_funds` below zero fails instead of
succeeding, even if the contract's total balance for that denom would
otherwise cover it.

In the healthy case, a distribution never comes close to this cap, since all
reward math floors in the recipient's favor (i.e. against the distribution).

Distributions created before this cap was introduced have no `claimable_funds`
tracked (`null`/`None`) and are not capped, since it isn't possible to know
how much of their already-funded amount is still outstanding after the fact.
`UnsafeForceWithdraw` is intentionally unsafe and uncapped, and does not
affect `claimable_funds`.
