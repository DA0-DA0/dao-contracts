# Gauge Budget Allocator

A minimal example of a [gauge adapter](../gauge-adapter/README.md) that
distributes a fixed native-token budget proportionally to gauge weights.
Compared to the [marketing-gauge adapter](../gauge-adapter/README.md), this
contract:

- has **no submission registry** — the admin curates the option list directly,
- has **no bond** — no `required_deposit` / `ReturnDeposits` flow,
- handles **native tokens only** — no cw20 path.

It exists as a working second example for the orchestrator + adapter pattern
and as a starting point for adapters that need a similar minimal shape
(treasury allocation, validator-preference signaling, AMM-incentive routing,
etc.). The contract is ~150 lines of logic — fork it freely.

## Lifecycle

1. **Instantiate.** `admin`, an initial `options` list (non-empty), and an
   `epoch_budget: Coin`.
2. **Manage options.** Admin can `AddOption`, `RemoveOption`, or
   `UpdateBudget` at any time.
3. **Wire into a gauge.** The DAO creates a gauge against this adapter via
   `gauge-orchestrator`'s `CreateGauge`. The orchestrator pulls
   `AllOptions` on attach and queries `CheckOption` when voters propose
   new options via `ExecuteMsg::AddOption` upstream — those proposals
   need to match the admin-curated list here.
4. **Execute.** At epoch close the orchestrator queries
   `SampleGaugeMsgs { selected }`, which returns `BankMsg::Send` payouts
   for each `(recipient, weight)` pair sized as
   `epoch_budget.amount * weight` (floor).

## ExecuteMsg

| Variant | Auth | Notes |
|---|---|---|
| `AddOption { option }` | admin | Reject if option already in the set. |
| `RemoveOption { option }` | admin | Reject if option not in the set. |
| `UpdateBudget { epoch_budget }` | admin | Replaces the per-epoch budget. |

## QueryMsg

| Variant | Response | Purpose |
|---|---|---|
| `Config {}` | `Config { admin, epoch_budget }` | Inspect deployed parameters. |
| `AllOptions {}` | `AllOptionsResponse` | Used by the orchestrator on attach. |
| `CheckOption { option }` | `CheckOptionResponse` | Used by the orchestrator on `AddOption`. |
| `SampleGaugeMsgs { selected }` | `SampleGaugeMsgsResponse` | Translates a selected set into payouts. |

## Errors

| Variant | Trigger |
|---|---|
| `Unauthorized` | Non-admin called a mutating message. |
| `OptionAlreadyExists(option)` | `AddOption` with an option already in the set. |
| `OptionDoesNotExist(option)` | `RemoveOption` on an option not in the set. |
| `NoOptions` | Instantiated with an empty `options` list. |

## Funding

The contract itself does not hold the budget — the DAO does. The
orchestrator dispatches `BankMsg::Send` messages from the DAO's core
module, so the DAO must hold enough of `epoch_budget.denom` for the
selected set to actually transfer. If the DAO is underfunded the dispatch
will fail at execution time; consider integrating with a treasury balance
check or topping the DAO up regularly.
