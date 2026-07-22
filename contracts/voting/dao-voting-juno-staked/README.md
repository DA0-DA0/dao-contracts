# dao-voting-juno-staked

A thin DAO DAO voting module that reads staked-JUNO voting power from Juno's
chain-owned `x/voting-snapshot` module.

## Compatibility

This contract is Juno-specific. It requires the Juno v30 wasm custom-query
bindings and is intended for the v30 `uni-7` network. Chains and older Juno
releases without `x/voting-snapshot` cannot answer its queries.

The chain owns voting-power policy and storage:

- DAO DAO treats height `h` as beginning-of-block voting power. Juno persists
  settled staking snapshots in EndBlock, so both voting-power queries translate
  DAO height `h` to Juno snapshot height `h - 1` and return the requested DAO
  height in the response. This keeps proposal totals and later voter queries on
  one immutable basis even when staking changes later in the proposal block.
- Omitting `height` uses the beginning of the current block (the previous
  settled Juno snapshot).
- Liquid-staking exclusion applies only to addresses in Juno's
  governance-managed LST allowlist. Juno v30 defaults this allowlist to empty,
  so no address is excluded until governance configures it. Operators must
  verify the live parameter; v30 does not generically detect liquid-staking
  contracts.
- Juno v30 defaults snapshot retention to `0` (pruning disabled). Governance can
  change both retention and the LST allowlist after deployment, so neither
  default should be assumed from the binary alone.

These historical semantics let DAO DAO proposal modules use power fixed at a
proposal's snapshot height, rather than recomputing from current stake.

## Activation and migration boundaries

Juno seeds its first voting snapshots at the v30 upgrade/backfill height `U`.
Because DAO height `h` reads Juno snapshot `h - 1`, the first DAO height backed
by that seed is `U + 1`; queries at `h <= U` can return zero. Before activating
this voting module:

1. confirm the v30 upgrade/backfill block has committed;
2. wait until at least `U + 1`;
3. query `/juno/votingsnapshot/v1/params` and record the live LST allowlist and
   retention window; and
4. do not switch an existing DAO while proposals needing pre-activation voting
   history remain open.

An exported-genesis restart similarly does not preserve earlier snapshot
history. Treat the restart seed height as a new activation boundary and do not
carry open proposals that require pre-restart power through such a switch.

This artifact is **fresh-deployment-only with respect to the unreleased
hook-enabled PR builds**. Those builds used the same `2.8.0-alpha.2` CW2 version
while exposing different state and wire behavior, so the normal migration guard
cannot identify them as older releases. Do not migrate an instance created from
those commits; instantiate this module fresh. The migrate entry point is
reserved for explicitly versioned, state-compatible future releases with the
same CW2 contract identity.

## Deploy and instantiate

Store the wasm on a compatible Juno v30 chain, then instantiate it from the DAO
core with the core contract as admin. The instantiate payload is empty:

```json
{}
```

Configure the resulting address as the DAO's voting module. Users continue to
delegate, undelegate, and redelegate through Juno `x/staking`; this contract has
no user execute operations. Its standard DAO DAO query surface is:

- `VotingPowerAtHeight`
- `TotalPowerAtHeight`
- `Dao` (the DAO core that instantiated this voting module)
- `Info`

The release-candidate workflow has been exercised on Juno v30 `uni-7`: store,
instantiate, all four DAO DAO queries, and voter/total-power behavior across a
real staking change and EndBlock boundary. Release operators must still bind
that smoke evidence to the exact artifact checksum they intend to deploy.

## Why staking-delta hooks are not exposed

Synchronous hook fanout cannot be implemented correctly with Juno v30's query
API. `x/cw-hooks` invokes contract sudo synchronously before
`x/voting-snapshot` persists dirty snapshots in EndBlock, so a current-height
query from that sudo sees the prior settled value. Delegation-removal events
also do not identify a delegator's remaining power across other validators,
and slash or validator bond-status changes can affect many delegators without
enumerating them.

Consequently this module deliberately exposes no cw-hooks registration, sudo
staking-event interface, subscriber management, or per-event DAO hooks.
Downstream consumers must query voting power at the historical height they need
and must not expect lossless per-delegator callbacks from this module.

## Limitations

- The module is usable only where Juno's v30 custom wasm query API is present.
- Current queries deliberately use the previous settled Juno snapshot so DAO
  height semantics remain beginning-of-block and stable throughout the block.
- Voting-snapshot retention must exceed the maximum proposal lifetime plus an
  operational margin. Juno v30 defaults retention to `0` (no pruning), but
  governance can enable a finite window. Pruning retains a carry-forward anchor;
  queries older than that retained anchor can still return zero.
- The `Dao` query returns the instantiating address recorded at setup; changing
  the wasm admin does not change which DAO the voting module belongs to.
