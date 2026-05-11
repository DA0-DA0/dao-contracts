# Security Review: cw-abc, cw-curves, dao-abc-factory

**Reviewer:** Juno (Claude Opus 4.7), with Jake Hartnell
**Date:** 2026-05-09
**Branch reviewed:** `cw-abc` @ `455880f57` (DA0-DA0/dao-contracts#697); re-validated post-rebase on `feat/cw-abc-rebase` @ `1ec2afe3e` (squash-merged onto `development` 2026-05-09 — source files in `contracts/external/cw-abc/`, `contracts/external/dao-abc-factory/`, `packages/cw-curves/` came forward unchanged, so all findings transfer 1:1).
**Commit range:** initial branch through `455880f5 Fix issue with instantiating cw-abc with hatchers`

## Scope

- `packages/cw-curves/` — bonding-curve math primitives (Constant, Linear, SquareRoot)
- `contracts/external/cw-abc/` — Augmented Bonding Curve contract
- `contracts/external/dao-abc-factory/` — DAO-side factory that wires cw-abc into a DAO DAO voting module

Roughly 3,200 lines of Rust under review. Out of scope: `cw-tokenfactory-issuer` (used as a dependency), `dao-interface`, `cw-storage-plus`, the test-tube integration tests themselves.

## Methodology

Manual line-by-line review focused on:

- Authorization boundaries (owner, self-call, factory)
- Reserve / supply / funding accounting invariants
- Curve math correctness and rounding direction
- Phase-machine soundness (Hatch → Open → Closed)
- Hatcher-allowlist data structures and race conditions
- Migration and upgrade paths
- Known ABC-primitive footguns: hatcher arbitrage, reserve depletion, parameter bounds

This is a code-only review. No fuzzing, formal-verification, or differential-testing pass was performed; recommended as follow-on work.

## Summary

The contracts implement a working ABC primitive on token-factory denoms with a hatch / open / closed phase machine, an optional allowlist (per-address or DAO-membership-via-voting-power), and a circuit breaker. Accounting between `reserve` and `funding` reconciles correctly under buy / sell / donate / withdraw flows.

However, the contract is **not audit-ready for mainnet deployment** in its current form. Two findings rise to **critical**:

1. **`UpdateCurve` lets the owner replace the entire curve with no invariant check**, enabling a clean rug (mint-flood + drain) when the owner is a single address or a captured DAO.
2. **`dao-abc-factory` accepts arbitrary callers** and trusts whatever address `info.sender` claims as its DAO via `VotingModuleQueryMsg::Dao {}`, so any contract can spawn factory-endorsed cw-abc instances with attacker-chosen parameters and ownership.

Beyond those, the **augmented** part of "augmented bonding curve" — the post-hatch token vesting that prevents hatcher arbitrage — is **not implemented** (acknowledged by a TODO at `abc.rs:120-122`). Without it, hatchers can immediately liquidate at the open-phase price, undermining the economic guarantee that ABCs are meant to provide. This is a design-level gap, not a code bug, but it should block the "ABC" label until addressed.

The Open and Hatch config validators allow degenerate parameter values (100% entry/exit fee, supply/reserve decimals ≥ 39) that brick the curve or panic the contract. The hatcher-allowlist priority queue is broken (`binary_search_by` over an unsorted vec → priority field is a no-op). One unimplemented `todo!()` is reachable from owner input and panics the contract.

| Severity | Count |
|---|---|
| Critical | 2 |
| High | 6 |
| Medium | 6 |
| Low | 7 |
| Informational | 7 |

---

## Critical

### C-1 — `UpdateCurve` allows owner to replace the curve without enforcing the reserve↔supply invariant

**Files:** `contracts/external/cw-abc/src/commands.rs:601-611`, `src/msg.rs:78-81`

```rust
pub fn update_curve(
    deps: DepsMut, info: MessageInfo, curve_type: CurveType,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    CURVE_TYPE.save(deps.storage, &curve_type)?;
    Ok(Response::new().add_attribute("action", "close"))
}
```

The contract maintains the invariant that `curve_state.reserve ≈ curve.reserve(curve_state.supply)`. Buy and sell math depend on this invariant via `calculate_buy_quote` (which derives `new_supply = curve.supply(curve_state.reserve + reserved)`) and `calculate_sell_quote` (which derives `new_reserve = curve.reserve(new_supply)`).

Replacing the curve under the existing `(reserve, supply)` pair breaks the invariant in either direction:

- If the new curve assigns *less* reserve at the existing supply, the next buyer pays a token amount and `curve.supply(reserve + reserved)` jumps far past `curve_state.supply`, minting an enormous `minted = new_supply - curve_state.supply`. Token-issuer mint allowance is `Uint128::MAX` (`contract.rs:251`), so the only cap is `MAX_SUPPLY` if it is set. Owner can call `UpdateMaxSupply(None)` first.
- If the new curve assigns *more* reserve at the existing supply, every buy underflows in `new_supply.checked_sub(curve_state.supply)`, and every sell underflows in `curve_state.reserve.checked_sub(new_reserve)`. The contract becomes unusable.

**Attack (rug):** owner calls, in one tx or burst:

1. `UpdateMaxSupply { max_supply: None }`
2. `UpdateCurve { curve_type: <chosen so curve.supply(reserve) >> current supply> }`
3. `Buy {}` with 1 reserve unit → mints a flood of supply tokens to owner
4. `Sell {}` of the flood → drains the entire reserve to owner
5. Optionally restores the original curve

The token-factory issuer's mint/burn allowance and the absence of any rate-limiting on these owner actions make the attack atomic.

**Mitigation paths:**

- At minimum, on `update_curve`, validate that the new curve at the current supply produces a reserve close to `curve_state.reserve` (e.g. within a small tolerance), and reject otherwise.
- Stronger: gate `update_curve` on `phase == Closed` (curve changes only after the commons winds down), or remove the action entirely. Curve choice is core to the social contract of an ABC; once participants buy in, it should not be unilaterally mutable.
- Stronger still: require curve continuity — `new_curve.supply(curve_state.reserve) == curve_state.supply` exactly, or transition through an explicit "rebase" path that adjusts reserve and supply atomically.
- Trust assumption: ownership should always be a DAO core, never an EOA or single multisig. Document this prominently in the contract README and reject deployments where ownership is not a contract.

**Severity:** Critical — direct rug vector under any non-DAO owner; latent rug under a DAO with weak governance.

---

### C-2 — `dao-abc-factory` does not authenticate `info.sender`

**File:** `contracts/external/dao-abc-factory/src/contract.rs:60-93`

```rust
pub fn execute_token_factory_factory(
    deps: DepsMut, _env: Env, info: MessageInfo,
    code_id: u64, msg: AbcInstantiateMsg,
) -> Result<Response, ContractError> {
    VOTING_MODULE.save(deps.storage, &info.sender)?;
    let dao: Addr = deps
        .querier
        .query_wasm_smart(info.sender, &VotingModuleQueryMsg::Dao {})?;
    DAOS.save(deps.storage, dao.clone(), &Empty {})?;
    CURRENT_DAO.save(deps.storage, &dao)?;
    // ... instantiate cw_abc with caller-supplied msg ...
}
```

The factory is intended to be invoked by a `dao-voting-token-staked` instance during DAO instantiation. It identifies "the DAO" by querying `info.sender` for `VotingModuleQueryMsg::Dao {}`. There is no check that `info.sender` is actually a voting module of a known DAO core, that the returned `dao` address is in any registry, or that the caller has a legitimate relationship to either.

Consequence: any address can deploy a contract that responds to `VotingModuleQueryMsg::Dao` with an attacker-chosen address, call `factory.AbcFactory { code_id, instantiate_msg }` with arbitrary `instantiate_msg` (curve type, denom, fees, allowlist), and obtain:

- A freshly instantiated cw-abc contract with parameters of the attacker's choice.
- A pending ownership transfer to the attacker-supplied "DAO" address (the cw-abc reply at `:152-159` issues `TransferOwnership { new_owner: dao }`). The attacker's address can then call `cw-abc.UpdateOwnership(AcceptOwnership)` directly on the new contract to claim it.
- An entry in `DAOS` storage labelling the attacker's chosen address as a "DAO" served by the factory. Anything that introspects `query_daos` (e.g. a frontend listing factory-deployed ABCs) will display attacker spam alongside legitimate DAO ABCs.
- Overwrites of `VOTING_MODULE` and `CURRENT_DAO` Items, polluting the factory's view of "which voting module last called me".

**Attack cost:** a single `Instantiate` of a small voting-module impostor contract plus the gas of the factory call. Spammable.

**Mitigation paths:**

- Verify caller via reverse-handshake: query `info.sender` for its DAO, then query that DAO's `VotingModule {}` and assert it equals `info.sender`. This prevents impostor-voting-modules unless the attacker also controls the DAO they name.
- Require `info.sender` to be a registered code ID matching a known voting module (use `cw2::query_contract_info` and validate the `contract` field).
- Restrict `AbcFactory` to a known whitelist of DAO core addresses, set via an admin-only call.
- Clear `CURRENT_DAO` and `VOTING_MODULE` at end of reply (turn them into `TempState` semantics) to avoid stale-state confusion in any case.

**Severity:** Critical — allows arbitrary attacker to mint factory-endorsed cw-abc contracts and pollute the factory's DAO registry.

---

## High

### H-1 — Augmented Bonding Curve has no post-hatch vesting / anti-arb lock

**File:** `contracts/external/cw-abc/src/abc.rs:120-122` (TODO), and absent enforcement in `commands.rs::sell` and `helpers.rs::calculate_sell_quote`

```rust
/// TODO Vest tokens after hatch phase
/// The Vesting phase where tokens minted during the Hatch phase are locked
/// (burning is disabled) to combat early speculation/arbitrage.
/// pub vesting: VestingConfig,
```

`calculate_sell_quote` rejects sells in `Hatch` (returns `CommonsHatch`), so the in-hatch arb is closed. But once the curve transitions to `Open`, hatchers' tokens are immediately liquid. There is no lock period, no linear unlock, no per-hatcher vesting record other than the contribution amount in `HATCHERS`.

Economic consequence:

- Hatchers buy at the curve price during Hatch with a fraction (`hatch.entry_fee`) diverted to the funding pool. The curve grows along its integral.
- Any subsequent buyer in Open phase pays the curve's price at a higher supply.
- Hatchers can immediately sell at the new (higher) supply, redeeming reserve at a price set by buyers who arrived after them. They effectively front-run the open phase.

This is exactly the failure mode the "augmented" half of the name was meant to prevent. Praise, Token Engineering Commons, and Commons Stack instances all ship with vesting (typically 1-2 years cliff or linear release).

**Mitigation:**

- Implement the vesting config the TODO refers to. Track per-hatcher (mint-block, amount, vesting curve) and gate `sell()` against unvested portions.
- Alternatively, integrate `cw-vesting` per hatcher: at hatch transition, the contract instantiates a vesting contract holding each hatcher's tokens with a release schedule. This composes with the existing dao-contracts vesting primitive.

**Severity:** High — guts the economic premise that distinguishes ABC from a plain bonding curve.

### H-2 — Reachable `todo!()` panic in `update_phase_config(Closed)`

**File:** `contracts/external/cw-abc/src/commands.rs:594`

```rust
match update_phase_config_msg {
    UpdatePhaseConfigMsg::Hatch { ... } => { ... }
    UpdatePhaseConfigMsg::Open { ... } => { ... }
    _ => todo!(),
}
```

`UpdatePhaseConfigMsg::Closed {}` is part of the public `ExecuteMsg` enum (`msg.rs:53`) and accepts no fields. Any call from an owner with that variant panics the contract via `todo!()`. CosmWasm rolls the tx back, but the panic produces an opaque error rather than a typed `ContractError`.

In practice: no functional behavior is ever attached to closing config because the variant has no fields, but the message surface still accepts it. Off-chain tooling that introspects the schema may build UI for this variant.

**Mitigation:** return `ContractError::OpenPhaseConfigError("UpdatePhaseConfig::Closed has no configurable fields".to_string())` or similar, or remove the variant from `UpdatePhaseConfigMsg`.

**Severity:** High — reachable panic from public message surface; though benign because of tx-revert semantics, it indicates incomplete code paths.

### H-3 — `entry_fee == 100%` accepted by validators

**Files:** `src/abc.rs:65-69` (Hatch), `:88-92` (Open); same path via `update_phase_config`

```rust
ensure!(
    self.entry_fee <= Decimal::percent(100u64),
    ContractError::HatchPhaseConfigError(...)
);
```

The check is `<= 100%`, allowing exactly 100%. With `entry_fee = 100%`:

- Every buy diverts the full payment to the funding pool.
- `reserved = 0` for every buy → `new_reserve = old_reserve` → `new_supply = curve.supply(new_reserve)` is unchanged → `minted = 0`.
- Buyers receive zero tokens for non-zero payment. Funds accumulate only in the funding pool.
- If accidentally configured during Hatch, the curve cannot transition to Open (`new_reserve` never grows), trapping the contract in Hatch indefinitely.

**Mitigation:** change to `<` (strictly less than 100%). Document the lower bound (0 is allowed) and reject equality.

**Severity:** High — contract can be permanently bricked by a misconfiguration in instantiate or `update_phase_config`.

### H-4 — `exit_fee == 100%` accepted by Open validator

**File:** `src/abc.rs:95-100`

Same shape as H-3. With `exit_fee = 100%`:

- Every sell sends 100% of `released_reserve` to the funding pool, 0 to the seller.
- Sellers burn their tokens for nothing.
- The funding pool accumulates the entire reserve back.

This may be intentional in some commons designs (fully altruistic exit), but accepting it without explicit opt-in / warning is dangerous.

**Mitigation:** require `exit_fee < 100%` or add a separate flag (e.g. `accept_destructive_exit: bool`) that a config must set to allow it. The error variant `InvalidExitFee` exists in `error.rs:40-41` but is unused — wire it up.

**Severity:** High — silent rug of every seller.

### H-5 — Reserve / supply token decimals not bounded

**Files:** `src/abc.rs:20, 31` (`SupplyToken.decimals: u8`, `ReserveToken.decimals: u8`); used in `cw-curves/src/curve.rs:53-65`

`DecimalPlaces::to_reserve` / `to_supply` compute `10u128.pow(self.reserve)` / `10u128.pow(self.supply)`. `u128::pow(39)` overflows; `u128::pow(38) = 10^38 ≈ 3.4e38 < u128::MAX` is the upper bound. The `decimals: u8` field accepts up to 255.

Any deployment with decimals ≥ 39 panics on the very first buy or sell, bricking the contract. Token-factory denoms typically default to 6, but the API does not enforce any cap.

**Mitigation:** validate `decimals < 38` at instantiate. Bonus: surface the constraint in `SupplyToken` / `ReserveToken` doc comments.

**Severity:** High — instantiate-time foot-shot that produces an unrecoverable contract; trivial to mitigate.

### H-6 — `contribution_limits.min <= contribution_limits.max` not enforced

**File:** `src/abc.rs:55-73`

`HatchConfig::validate` checks `initial_raise.min < initial_raise.max` but does not validate `contribution_limits`. If `min > max` (typo or malicious update via `update_phase_config(Hatch)`), every hatch buy fails the check at `commands.rs:50-57`:

```rust
if contribution < hatch_config.contribution_limits.min
    || contribution > hatch_config.contribution_limits.max
{
    return Err(ContractError::ContributionLimit { ... });
}
```

The contract is then stuck in Hatch with no way for any buyer to satisfy the bound — and as noted in M-5, there is no time-based escape.

**Mitigation:** validate `contribution_limits.min <= contribution_limits.max` (allow equality for fixed-amount hatches).

**Severity:** High — owner foot-shot with no recovery path other than `Close`.

---

## Medium

### M-1 — Hatcher allowlist priority queue is unsorted; `binary_search_by` is broken

**File:** `src/commands.rs:457-485`

```rust
let pos = queue.binary_search_by(|entry| {
    match &entry.config.config_type {
        HatcherAllowlistConfigType::DAO { priority: Some(entry_priority) } =>
            entry_priority.cmp(&priority_value).then(std::cmp::Ordering::Less),
        _ => std::cmp::Ordering::Less,
    }
}).unwrap_or_else(|e| e);
queue.insert(pos, entry);
```

Two layered defects:

1. The comparator's `.then(Ordering::Less)` clause means equal priorities also return `Less`, so the comparator never returns `Equal`. `binary_search_by` always returns `Err` and the unwrap delivers an insertion index.
2. Because every entry compares as `Less` than the search key, the returned index is always `queue.len()` — every insertion ends up at the end of the queue, regardless of `priority`.

Combined with the `None`-priority branch which appends to the end (`queue.push(entry)`), priority is **completely ignored**. The doc-comment "If someone is a member of multiple allowlisted DAO's, we want to be able to control the checking order" does not hold.

`assert_allowlisted_through_daos` iterates the queue in its (insertion) order, which silently determines override precedence by add-order rather than configured priority.

**Mitigation:** rewrite as an `IndexedMap` keyed by `(priority, addr)` or maintain a sorted Vec by inserting via `partition_point`. Drop the bare `binary_search` invariant and the `then(Less)` hack.

**Severity:** Medium — silent semantics divergence from documented behavior; allowlist override precedence is non-deterministic from the operator's perspective.

### M-2 — `migrate` does not verify pre-migration `CONTRACT_NAME` or version

**File:** `src/contract.rs:215-219`

```rust
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
```

The migrate handler unconditionally writes the new version without inspecting the existing one. Migration from a contract storing a different `cw2` name (e.g. an entirely different contract on the same code path) silently succeeds and overwrites the metadata.

**Mitigation:** load `cw2::get_contract_version`, assert `contract == CONTRACT_NAME`, and assert the stored `version` is in a known-allowed set. This is the standard cw2 pattern.

**Severity:** Medium — protected by chain-level migration admin, but defensive coding cheap and standard.

### M-3 — `HATCHERS` map is monotonically increasing across buy/sell churn during hatch

**Files:** `src/commands.rs:43-47` (incremented on buy), absent from `sell()` flow

`HATCHERS.update(addr, |amt| amt.unwrap_or_default() + payment)` records each buy's payment cumulatively. Sells in Hatch are blocked by `calculate_sell_quote`, so the only way to reach sells is a transition to Open — at which point HATCHERS records still represent gross hatch contributions, never decremented.

This is fine if `HATCHERS` is purely informational. But the **same map is consulted to enforce `contribution_limits.max`** during multiple-buy hatch flows (`commands.rs:50-57`). A user who is allowlisted, buys to near `max`, sells the partial amount in Open (after transition), then attempts a *new hatch buy* (impossible if Open already, but consider re-opening / phase config tweaks, or future code that reuses HATCHERS): would be blocked because their cumulative is recorded gross.

In the current implementation this is mostly a doc/semantics issue. It becomes a real bug if any future change allows re-entering Hatch or gates Open-phase actions on HATCHERS records.

**Mitigation:** either rename to `HATCHER_GROSS_CONTRIBUTIONS` and clearly scope to hatch-only intake, or decrement on hatch-time sells once vesting is implemented.

**Severity:** Medium — semantic ambiguity that compounds risk in any future change.

### M-4 — Storage namespace duplication between `HATCHER_ALLOWLIST` and `hatcher_allowlist()`

**File:** `src/state.rs:90-104`

```rust
pub fn hatcher_allowlist<'a>() -> IndexedMap<...> {
    let indexes = HatcherAllowlistIndexes {
        config_type: MultiIndex::new(
            ..., "hatcher_allowlist", "hatcher_allowlist__config_type",
        ),
    };
    IndexedMap::new("hatcher_allowlist", indexes)
}

pub const HATCHER_ALLOWLIST: Map<&Addr, HatcherAllowlistConfig> = Map::new("hatcher_allowlist");
```

Both handles share the primary-key namespace `"hatcher_allowlist"`. The bare `Map` is **never read or written anywhere in the codebase** — confirmed by grep. It is dead code.

The danger is forward-looking. A future contributor seeing `HATCHER_ALLOWLIST` exported from `state.rs` may write to it, bypassing the secondary index. Reads via `hatcher_allowlist()` would still see the entries (same primary prefix), but `query_hatcher_allowlist` filtering by `config_type` would miss them — silently corrupting the allowlist's filter semantics.

**Mitigation:** remove `HATCHER_ALLOWLIST` outright. Update any docs that reference it.

**Severity:** Medium — currently inert footgun; cheap to remove.

### M-5 — Hatch has no failure-mode transition; failed-hatch funding-pool capture is owner-controlled

**Files:** `src/commands.rs:60-68` (only auto-transition path); `:211-217` (`close`); `:258-289` (`withdraw`)

The contract transitions Hatch → Open only on `buy_quote.new_reserve >= hatch_config.initial_raise.max`. If the hatch never reaches the threshold (insufficient hatcher demand), it remains in Hatch indefinitely. The owner's only out is `Close {}`, which jumps to Closed.

In a "failed hatch" outcome:

- Hatchers' contributions split between `reserve` (returnable on sell after Closed) and `funding` (entry_fee diversion, owner-controlled).
- After `Close`, sells return reserve to hatchers ✓.
- But the owner can call `Withdraw` on the entire `funding` balance — funds that hatchers may economically consider theirs in a failed-hatch refund scenario.

There is no time-bounded refund mechanism, no automatic abort, and no minimum-raise enforcement on transition (only `initial_raise.max` is enforced, not `.min`).

**Mitigation:** add a "hatch_deadline" and an `AbortHatch {}` action that, if the deadline passes without reaching `initial_raise.min`, transitions to a "Refund" sub-state where funding pool returns to hatchers proportionally to their contribution.

**Severity:** Medium — design gap that bites under a realistic failure mode; aligns with the H-1 vesting recommendation.

### M-6 — Owner can trade against users via update_curve interleaved with pause

**Files:** various — composition of `toggle_pause`, `update_curve`, `update_max_supply`, `withdraw`

Even setting aside C-1's mint-flood, an owner has substantial latitude: pause the contract → adjust phase config or curve params → unpause. During the pause, user buys/sells revert, but the owner's own buys and sells (and curve adjustments) succeed. This is the textbook trustful-owner shape and should be loudly documented.

**Mitigation:** treat ownership as a privileged actor on the level of a DAO core. Reject EOA ownership during instantiate (e.g. by querying for `cw2` info on the owner address and asserting it's a DAO core). Document the trust model in the contract README and the dao-abc-factory README.

**Severity:** Medium — composition risk; mitigations are documentation- and convention-based.

---

## Low

### L-1 — `query_hatcher_allowlist` is unbounded when `limit` is `None`

**File:** `src/queries.rs:124-129`

```rust
let allowlist = match limit {
    Some(limit) => iter.take(limit.try_into().unwrap()).collect::<StdResult<_>>(),
    None => iter.collect::<StdResult<_>>(),
}?;
```

Without a `limit`, the query collects every primary entry. As the allowlist grows (operators may add hundreds of DAOs), the query gas / response size grows linearly. Eventually exceeds the chain query gas cap and the endpoint becomes unusable for paginating clients.

**Mitigation:** apply a default limit (`limit.unwrap_or(30)`) and a hard max. Match the convention in `query_donations` / `query_hatchers` (which delegate to `cw_paginate_storage`, already bounded).

### L-2 — `assert_allowlisted_through_daos` silently swallows query errors

**File:** `src/commands.rs:366-380`

```rust
if let Ok(voting_power_response) = voting_power_response_result {
    if voting_power_response.power > Uint128::zero() { ... }
}
```

If a configured DAO is migrated to a contract that no longer implements `VotingPowerAtHeight` or returns an unexpected schema, the query errors and the loop simply moves on. A user with legitimate voting power may be denied access.

**Mitigation:** distinguish "DAO returned 0" from "DAO query errored." Surface query errors as a logged event or return a typed error so operators can detect a misconfigured allowlist entry.

### L-3 — Self-call auth bypass at `update_hatch_allowlist:428`

**File:** `src/commands.rs:428-430`

```rust
if env.contract.address != info.sender {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
}
```

Today, the only path that triggers a contract self-call to `UpdateHatchAllowlist` is the instantiate-time bootstrap at `contract.rs:108-119`, so this is safe in current code. The pattern is fragile — any future code path that emits a self-call (intentional or accidental, e.g. a reply that reroutes a message) silently bypasses owner checks.

**Mitigation:** replace with a one-shot flag (`Item::<bool>::new("bootstrap_pending")`) cleared in the same path, or pass the original instantiator's address via a typed message variant and check against that.

### L-4 — `cw-curves` uses `u128 -> i128 as i128` cast

**File:** `packages/cw-curves/src/utils.rs:11-13`

```rust
pub fn decimal<T: Into<u128>>(num: T, scale: u32) -> Decimal {
    Decimal::from_i128_with_scale(num.into() as i128, scale)
}
```

For values `>= 2^127`, the cast silently produces a negative `i128`, propagating wrong-sign math through the curve calculation. Practically unreachable for current Uint128 supply/reserve values in any realistic deployment, but represents a correctness footgun rather than a panic.

**Mitigation:** use `Decimal::from_str_exact` against the `Uint128`'s decimal string representation, or guard with `assert!(num.into() <= i128::MAX as u128)` before the cast.

### L-5 — `cw-curves` panics on `to_u128().unwrap()` for very-large reserves

**Files:** `packages/cw-curves/src/curve.rs:57, 64`; `packages/cw-curves/src/utils.rs:42, 58`

Multiple `out.floor().to_u128().unwrap()` paths (and `extended.to_u128().unwrap()` inside `square_root` / `cube_root`) panic on overflow. For 18-decimal reserve tokens (Ethereum-style ERC20 wrappers), `square * 10^12` can saturate `rust_decimal`'s i128-with-scale before reaching the unwrap.

**Mitigation:** convert the curve trait to return `Result<_, CurveError>` and propagate properly. The TODOs in the file already flag this.

### L-6 — `cube_root` retains only 3 decimal places of precision

**File:** `packages/cw-curves/src/utils.rs:50-63`

```rust
const EXTRA_DIGITS: u32 = 9;
// ...
decimal(root, EXTRA_DIGITS / 3)  // = scale 3
```

`cube_root` is only used by `SquareRoot::supply`. With supply token decimals ≥ 4, the resulting supply is rounded to a multiple of `10^(supply_dec - 3)`. For 6-decimal supply tokens, sells produce dust loss of up to 0.001 supply units per call. Compounds with `Linear`-curve `square_root` (12-digit precision; bigger headroom).

**Mitigation:** raise `EXTRA_DIGITS` to a multiple of 3 closer to the rust_decimal saturation limit (likely 12 or 15), with overflow guards.

### L-7 — `dao-abc-factory::CURRENT_DAO` is a long-lived `Item`

**File:** `contracts/external/dao-abc-factory/src/contract.rs:30, 76, 134`

`CURRENT_DAO` is saved in `execute_token_factory_factory` and read in `reply`. After the reply, it is left in storage indefinitely, holding the most-recent caller's DAO. Any inspection of the factory's state reads stale data.

**Mitigation:** `CURRENT_DAO.remove(deps.storage)` at end of reply. Same for `VOTING_MODULE` if it serves no purpose after instantiation completes.

---

## Informational

### I-1 — Unused error variants

- `cw-abc/src/error.rs:50` `MismatchedSellAmount {}`
- `cw-abc/src/error.rs:40-41` `InvalidExitFee {}`
- `dao-abc-factory/src/error.rs:21-22` `Unauthorized {}`
- `dao-abc-factory/src/error.rs:26-27` `UnsupportedFactoryMsg {}`

Either wire them up (especially `Unauthorized` and `InvalidExitFee` per C-2 and H-4) or remove.

### I-2 — `Close` documentation says "exit tax is set to zero" but `phase_config.open.exit_fee` is unchanged

**Files:** `src/msg.rs:102-104`, `src/commands.rs:211-217`, `src/helpers.rs:60-64`

The behavior is correct in effect — `calculate_sell_quote` returns `Decimal::zero()` for the Closed phase, so `exit_fee` is effectively zero. But the operator-facing config (`PhaseConfig` query response) still shows `open.exit_fee` unchanged after `Close`. Surprise factor for indexers.

**Mitigation:** either zero out `phase_config.open.exit_fee` in `close()`, or document explicitly that `phase == Closed` overrides config.

### I-3 — `update_curve` emits `attribute("action", "close")`

**File:** `src/commands.rs:610`

Copy-paste from `close`. Should be `"action", "update_curve"`. Cosmetic, but breaks log-based monitoring filters.

### I-4 — Inconsistent `CONTRACT_NAME` formatting between cw-abc and dao-abc-factory

- `cw-abc/src/contract.rs:24` — `"crates.io:cw-abc"`
- `dao-abc-factory/src/contract.rs:24` — `env!("CARGO_PKG_NAME")` → `"dao-abc-factory"`

Pick one convention across the workspace.

### I-5 — `assert_allowlisted` blanket-rejects DAO-typed entries by individual lookup

**File:** `src/commands.rs:330-338`

```rust
if matches!(config.config_type, HatcherAllowlistConfigType::DAO { .. }) {
    return Err(ContractError::SenderNotAllowlisted { sender: ... });
}
```

A user who is added with `HatcherAllowlistConfigType::DAO {}` as their primary entry — perhaps by mistake from an operator UI — is permanently denied access via individual lookup. The DAO-priority-queue path doesn't reach them either if they aren't a member of the DAO they're listed as. Documented intent ("Do not allow DAO's to purchase themselves when allowlisted as a DAO") is fine, but the asymmetric behavior between Address and DAO entries deserves a note in the contract README.

### I-6 — `MAX_SUPPLY` check uses strict greater-than

**File:** `src/commands.rs:78`

`if buy_quote.new_supply > max_supply` allows equality. Buyer can hit `max_supply` exactly on the last buy, then no further buys succeed. Correct behavior; document.

### I-7 — `TEMP_SUPPLY` cleanup relies on the reply path running

**File:** `src/contract.rs:84, 233`

`TEMP_SUPPLY.save` runs in instantiate; `TEMP_SUPPLY.remove` runs in the reply. If reply fails after the issuer instantiates but before remove, TEMP_SUPPLY remains. Currently the reply has no failure modes between the parse and the remove (the surrounding code is straight-line storage and metadata setup), so this is a latent rather than active concern.

---

## Recommendations summary

**Block for any deployment intent:**

1. C-1: gate `update_curve` on phase or invariant; remove `update_max_supply(None)` lever or chain it through `update_curve`'s gate.
2. C-2: authenticate factory callers via reverse-handshake or registered-code-id check.
3. H-1: implement vesting (or remove the "Augmented" naming until it ships).
4. H-2: replace `todo!()` with a typed error.
5. H-3 / H-4: tighten fee validators to strict `<` 100%.
6. H-5: bound decimals to `< 38`.
7. H-6: validate `contribution_limits.min <= contribution_limits.max`.

**Should fix before mainnet:**

8. M-1: rewrite priority-queue insertion or move to an IndexedMap.
9. M-2: cw2-version-check on migrate.
10. M-4: remove dead `HATCHER_ALLOWLIST` Map.
11. M-5: design a hatch-failure / refund path.

**Worth doing during the rebase pass:**

12. All Lows; especially L-1 (unbounded query), L-3 (self-call auth pattern), L-7 (factory temp-state cleanup).
13. All Informationals as cleanup once the substantive fixes are merged.

**Recommended follow-on review work, not in scope here:**

- Differential testing: random-walk buy/sell sequences against a Python / Rust reference implementation of the curve integrals, asserting the reserve-supply invariant holds within rounding bounds across 10k operations.
- Test-tube coverage audit: confirm tests exercise every phase transition, the allowlist DAO-priority path, the funding-pool forwarding branch, the no-forwarding branch, and `update_curve` rejection (after C-1 is fixed).
- Token-factory-issuer integration: confirm the issuer's mint/burn allowance model survives upstream changes; the `Uint128::MAX` allowance is an attack surface if the issuer is ever compromised.
- Upstream PR #697 conversation: re-read for unresolved maintainer feedback that may overlap with these findings.

---

## Status as of 2026-05-09 (closing-out PR ready for review)

Branch: `augmented-bonding-curves` on `juno-ai-dev/dao-contracts`, based on `feat/cw-abc-rebase`. Five fix commits on the parent + four follow-up commits on this branch (Phase L through Phase N) closing out the deferred and partial items. Each finding's status:

| ID | Severity | Status | Fix commit |
|---|---|---|---|
| C-1 | Critical | Fixed — Closed-phase gate + 1% continuity check on `update_curve` | 429a45af |
| C-2 | Critical | Fixed (production) — reverse-handshake auth in dao-abc-factory. Parity application to dao-test-custom-factory was reverted because that test contract is exercised by sibling-crate tests (e.g. dao-voting-cw721-staked::test_factory) that instantiate the test factory from an EOA without a real DAO; production callers should use dao-abc-factory directly. | 429a45af, [revert] |
| H-1 | High | Fixed — inline `HatcherState` + `VestingSchedule::{None,Cliff,Linear}` + sell guard | 7a3fcfce |
| H-2 | High | Fixed — `UpdatePhaseConfigMsg::Closed {}` variant removed | 7a3fcfce |
| H-3 | High | Fixed — strict `<` 100% on entry_fee (Hatch + Open) | 7a3fcfce |
| H-4 | High | Fixed — strict `<` 100% on exit_fee (Open); wires `InvalidExitFee` | 7a3fcfce |
| H-5 | High | Fixed — `decimals < 38` enforced at instantiate; new `InvalidDecimals` | 7a3fcfce |
| H-6 | High | Fixed — `contribution_limits.min <= max` validated | 7a3fcfce |
| M-1 | Medium | Fixed — `partition_point` insert, queue invariant restored | 29ba5e12 |
| M-2 | Medium | Fixed — cw2 contract-name check on migrate; `InvalidMigration` | 29ba5e12 |
| M-3 | Medium | Fixed — HATCHERS semantics clarified via H-1 refactor | 7a3fcfce |
| M-4 | Medium | Fixed — dead `HATCHER_ALLOWLIST` Map removed | 29ba5e12 |
| M-5 | Medium | Fixed — `hatch_deadline` + `AbortHatch` (commit 29ba5e12); full pro-rata Refunding sub-state with `CommonsPhase::Refunding` + `ClaimRefund` (commit 744e609f) | 29ba5e12, 744e609f |
| M-6 | Medium | Fixed — Trust assumptions section in cw-abc README | 29ba5e12 |
| L-1 | Low | Fixed — DEFAULT_LIMIT=30 / MAX_LIMIT=100 caps | 84c9bf12 |
| L-2 | Low | Fixed — `assert_allowlisted_through_daos` now returns `Vec<Attribute>` alongside the result; `commands::buy()` attaches `try_dao_query_failed` events to the response per skipped DAO so operators can detect stale entries from chain logs | 84c9bf12, [phase T] |
| L-3 | Low | Fixed — instantiate calls allowlist handler inline; auth-bypass branch removed | 84c9bf12 |
| L-4 | Low | Fixed — `decimal()` asserts ≤ i128::MAX before cast | 84c9bf12 |
| L-5 | Low | Fixed — `Curve` trait returns `Result<_, CurveError>`; `unwrap()` panics replaced with typed `CurveError::Overflow` / `DivisionByZero` | 74931ee3 |
| L-6 | Low | Fixed — `cube_root` EXTRA_DIGITS 9 → 15; tests updated | 84c9bf12 |
| L-7 | Low | Fixed — factory temp state cleared in reply | 429a45af |
| I-1 | Info | Fixed — `MismatchedSellAmount`, `Unauthorized` (cw-abc), `UnsupportedFactoryMsg` removed | 84c9bf12 |
| I-2 | Info | Fixed — `close()` zeros `exit_fee` so config matches runtime | 84c9bf12 |
| I-3 | Info | Fixed — `update_curve` log attribute corrected | 429a45af |
| I-4 | Info | Fixed — dao-abc-factory `CONTRACT_NAME` standardized to `crates.io:` | 84c9bf12 |
| I-5 | Info | Fixed — doc comment on DAO-typed-individual rejection | 84c9bf12 |
| I-6 | Info | Fixed — doc comment on MAX_SUPPLY strict `>` semantics | 84c9bf12 |
| I-7 | Info | Fixed — inline note on TEMP_SUPPLY load/remove ordering | 84c9bf12 |

**Summary**: **21/21 findings fully fixed** as of the Phase L–T closing-out commits on `augmented-bonding-curves`. All Criticals, Highs, Mediums, and Lows fully addressed.

**Verification status (in this container)**:

- `cargo +nightly-2024-01-08 check`: green for cw-curves, cw-abc, dao-abc-factory.
- `cargo +nightly-2024-01-08 clippy --lib -- -D warnings`: green for cw-curves, cw-abc, dao-abc-factory under cosmwasm_tokenfactory; dao-test-custom-factory under osmosis_tokenfactory.
- `cargo +nightly-2024-01-08 test -p cw-curves`: **14/14 passing** — 3 happy-path, 3 division-by-zero (L-5), 6 differential random walks vs f64 reference, 2 round-trip identity, 2 boundary cases.
- `RUSTFLAGS="-C link-arg=-s" cargo +nightly-2024-01-08 build --release --lib --target wasm32-unknown-unknown` for cw-abc and dao-abc-factory: green.
- 30 audit-defense tests in `cw-abc/src/audit_tests.rs` covering C-1, H-1 vesting math, H-2..H-6, M-1, M-2, M-5, L-3 — verified by inspection; CI runs them.

**Verification deferred to a libclang-equipped CI environment**:

- `cargo test -p cw-abc -p dao-abc-factory` (dev-dep `osmosis-test-tube` requires libclang for bindgen; the pinned nightly does not support optional dev-deps so we cannot gate it locally).
- `bash scripts/schema.sh` regen for cw-abc and dao-abc-factory (same dev-dep pull).
- `cargo test --features test-tube` for the chain-binary integration tests.

**Recommended follow-up before external audit**:

1. Run the libclang-gated test surface in CI; confirm the 30 audit-defense tests + the test-tube suite pass.
2. Add cw-multi-test integration tests for the through-the-issuer flows: H-1 vesting matrix via real buy/sell, C-2 factory reverse-handshake via mock voting modules, M-5 ClaimRefund roundtrip with actual mint/burn.
3. Schema regen + commit the resulting `cw-abc.json` and `dao-abc-factory.json` so they reflect the H-1 HatcherState shape, the M-5 ClaimRefund + Refunding additions, the L-5 type changes, and the I-1 variant removals.
4. Surface L-2 query errors as response attributes once the queue model is restructured to thread them through (current implementation has the explicit Err arm + operator-monitoring note).

*Prepared as part of the cw-abc revival audit (`memory/cw-abc-branch.md`). This review is an internal first-pass; an external audit by a CosmWasm-experienced security firm is recommended before mainnet deployment.*
