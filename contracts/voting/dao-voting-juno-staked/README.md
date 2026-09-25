# dao-voting-juno-staked

A DAO DAO voting module that derives voting power from **staked JUNO**, using
Juno's `x/voting-snapshot` chain module for historical snapshots.

## What this fixes vs. `dao-voting-cosmos-staked`

The earlier attempt at a Cosmos-staked voting module
([DA0-DA0/dao-contracts#832](https://github.com/DA0-DA0/dao-contracts/pull/832))
lived inside contract storage with no access to historical staking data, and
[acknowledged in its README](https://github.com/DA0-DA0/dao-contracts/pull/832/files#diff-README)
that voting power "is always calculated based on the current amount staked
(regardless of which block is requested in the query)." That breaks any
proposal module that relies on a stable voting-power snapshot at proposal
creation — voters can rage-stake mid-proposal, slashing silently moves the
denominator, etc.

Juno v30 ships a chain module — `x/voting-snapshot` — that records
`(delegator, height) → bonded power` on every staking event with LST
exclusion built in. This contract is a thin consumer of that module:

- **`VotingPowerAtHeight`** asks the chain for the snapshot at the requested
  height (returning the at-or-before snapshot per the chain's semantics).
- **`TotalPowerAtHeight`** returns the total bonded supply at the same height.
- **Liquid-staked tokens contribute zero voting power** because the chain
  enforces the LST allowlist before writing the snapshot.

## Hook fan-out

For consumers that need stake-change notifications (the gauge orchestrator,
dao-rewards-distributor, etc.), this contract subscribes to Juno's
`x/cw-hooks` staking events via the standard `sudo` interface and re-emits
them as `dao_hooks::stake::StakeChangedHookMsg::{Stake, Unstake}` to all
registered subscribers. Subscribers register via `AddHook { addr }` (gated to
the DAO) and are auto-unregistered if their execute call errors (standard
`reply_on_error` pattern from the rest of the dao-contracts hook surface).

The amount in `Stake { addr, amount }` is the delegator's **new total bonded
power** read from `x/voting-snapshot` after the staking event lands, not the
delta of the individual delegation — this matches what gauge tally
maintenance actually needs ("what's this voter's new weight").

## Limitations

- **Juno-only.** This contract uses Juno's `x/voting-snapshot` custom wasm
  binding, which is not available on other Cosmos chains.
- **Requires v30 or later.** Older juno releases don't have the
  voting-snapshot module.
- **Staking happens at the SDK layer, not through this contract.** Users
  delegate via `MsgDelegate` / `MsgUndelegate` / `MsgBeginRedelegate`. This
  contract has no `Stake` / `Unstake` / `Claim` execute variants — the
  unbonding period is the chain's, not a contract config.
