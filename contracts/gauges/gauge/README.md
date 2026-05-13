# Gauge Orchestrator

A generic, stake-weighted preference signal that periodically translates
into on-chain action. The orchestrator hosts many gauges, each backed by a
pluggable [adapter](../gauge-adapter/README.md) that decides what the gauge
"means". Curve-inspired ([Curve gauges](https://resources.curve.fi/reward-gauges/gauge-weights));
see [`contracts/gauges/README.md`](../README.md) for the bigger picture.

## Lifecycle

1. **Attach.** The DAO calls `ExecuteMsg::CreateGauge` with a `GaugeConfig`
   that references an adapter address. The orchestrator queries the adapter's
   `AllOptions` to seed the option set, then opens it for voting.
2. **Vote.** Anyone with nonzero voting power calls `PlaceVotes` with a list
   of `(option, weight)` pairs whose weights sum to ≤ 1.0. The orchestrator
   walks each voter's previous vote and applies a tally diff in one pass.
3. **Update tallies on stake changes.** The orchestrator is registered as a
   staking hook (cw4 `MemberChangedHook`, cw20 `StakeChangedHook`, cw721
   `NftStakeChangedHook`). When a voter's power changes, every gauge they
   voted on is updated automatically — the user does not have to re-vote.
4. **Execute.** Once `next_epoch < block.time`, anyone can call
   `ExecuteMsg::Execute { gauge }`. The orchestrator snapshots the current
   tally, computes the *selected set* (top-N by weight subject to
   `min_percent_selected` and `max_available_percentage`), queries the
   adapter's `SampleGaugeMsgs(selected)` for the `CosmosMsg`s to dispatch,
   and forwards them to the DAO core for execution via
   `ProposalExecuteHook`. `next_epoch` advances by `epoch_size`.
5. **Optional reset.** If the gauge was created with a `reset_epoch`, the
   option list can be wiped and refreshed from the adapter on a separate
   cadence — useful for periodically pruning stale options without
   restarting the gauge.

## Why one orchestrator for many gauges

Each staking hook adds a CosmWasm call to every staking action. With N
separate gauge contracts each hooked, you pay N × hook-overhead per
stake/unstake. With one orchestrator, the hook fires once and the
orchestrator iterates its tallied state for each registered gauge — far
cheaper.

## Configuration knobs

Per-gauge config lives on the `Gauge` struct in `state.rs`. Mutable via
`ExecuteMsg::UpdateGauge` (owner-gated):

| Field | Meaning |
|---|---|
| `epoch` | Seconds between executions. Minimum 60. |
| `min_percent_selected` | Optional floor: options below this fraction of total cast are not selected. |
| `max_options_selected` | Hard cap on the size of the selected set. |
| `max_available_percentage` | Optional ceiling: an option's effective weight is clamped to this fraction (excess goes to no one). |
| `reset` | Optional periodic option-list refresh. |

## Adapter contract (`AdapterQueryMsg`)

Every adapter must answer:

| Query | Purpose |
|---|---|
| `AllOptions {}` | Initial option seed at gauge attachment. |
| `CheckOption { option }` | Validates user-proposed additions via `AddOption`. |
| `SampleGaugeMsgs { selected }` | Returns `Vec<CosmosMsg>` for the orchestrator to dispatch on the DAO's behalf. |

See [`gauge-adapter/README.md`](../gauge-adapter/README.md) for a worked
example.

## Voting power edge cases

Vote weight × voting power is computed with truncating integer math. A user
with 1 unit of power who splits 50/50 across two options would have *both*
options counted as 0 — silently erasing their voice. The orchestrator
rejects such votes with `VoteWeightRoundsToZero` so the user can retry with
larger per-option weights or fewer options. Acquire more power to express
finer-grained preferences.

## Storage layout

All non-global state is indexed first by `GaugeId` (u64, auto-incremented)
and then by a secondary key (voter address for votes, option string for
tallies). This is what lets one orchestrator host many gauges efficiently —
`.prefix()` / `.sub_prefix()` queries scope to a single gauge without
scanning the rest.

Key collections (see `state.rs`):

- `GAUGES: Map<GaugeId, Gauge>` — per-gauge config + execution state.
- `TALLY: Map<(GaugeId, &str), u128>` — cumulative weighted power per option.
- `OPTION_BY_POINTS: Map<(GaugeId, u128, &str), u8>` — secondary index for
  top-N selection.
- `TOTAL_CAST: Map<GaugeId, u128>` — denominator for percent math.
- `votes()` — indexed map keyed `(voter, gauge_id) → Vote`.

## Hooks the orchestrator must be registered against

`ExecuteMsg`:

- `StakeChangeHook` — cw20-staked, native-staked, token-factory-staked.
- `NftStakeChangeHook` — cw721-staked.
- `MemberChangedHook` — cw4 group changes.

The DAO's voting module (or its underlying staking contract) must add the
orchestrator address as a hook receiver; otherwise stake changes will not
flow into gauge tallies and the gauge will drift from reality.

## Hooks the orchestrator emits

The orchestrator broadcasts a `GaugeVoteHook` to every registered
subscriber on each `PlaceVotes` call. Subscribers receive the new vote
state — gauge id, voter, the new `Vec<Vote>` (empty on abstain), the
voter's `voting_power` at the snapshot, and `height`. Useful for
participation rewards (a sibling `dao-rewards-distributor` paying for
active gauge participation, off-chain analytics, notification routers,
etc.). The hook payload type is
[`hooks::GaugeVoteHookMsg`](src/hooks.rs); subscribers match on
`GaugeVoteHookExecuteMsg::GaugeVoteHook(..)`.

| ExecuteMsg | Auth | Notes |
|---|---|---|
| `AddHook { addr }` | owner | Add a subscriber. |
| `RemoveHook { addr }` | owner | Drop a subscriber. |

| QueryMsg | Returns | Notes |
|---|---|---|
| `GetHooks {}` | `GetHooksResponse { hooks: Vec<String> }` | List current subscribers. |

Subscriber failure is non-fatal to the voter: submessages use
`reply_on_error`, and the orchestrator's `reply` handler auto-drops a
failing subscriber by index so its broken callback can't keep blocking
gas on future votes. Adding a participation-reward consumer is therefore
safe to attempt — a misconfigured downstream contract is self-pruning.
