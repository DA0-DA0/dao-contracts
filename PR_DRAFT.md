# Revive `cw-abc` + ship `cw-curves` and `dao-abc-factory` with full audit pass

## Summary

This PR revives the [`cw-abc`](https://github.com/DA0-DA0/dao-contracts/tree/cw-abc) (Augmented Bonding Curve) branch — last touched in May 2024, ~2 years stale — by rebasing it onto `development` and shipping a complete fix for all 21 findings from a fresh internal security review.

It introduces three new workspace members:

- `packages/cw-curves` — bonding-curve math primitives (Constant, Linear, SquareRoot)
- `contracts/external/cw-abc` — the Augmented Bonding Curve contract
- `contracts/external/dao-abc-factory` — DAO-side factory wiring cw-abc into a DAO via `dao-voting-token-staked`

Includes an internal security review at [`audits/2026-05-09-cw-abc-security-review.md`](audits/2026-05-09-cw-abc-security-review.md) that documents the 21 findings, their fixes, and the remaining work for an external audit.

Closes (or builds on) the original draft: [#697](https://github.com/DA0-DA0/dao-contracts/pull/697).

## Why this exists

ABCs are a Commons-Stack-lineage primitive for community-funded project tokens — a bonding curve where buyers deposit a reserve asset and receive curve-priced tokens, augmented with a hatcher phase, a funding pool, and (now) inline vesting to combat early-arb. Used by Token Engineering Commons, Commons Stack, Giveth, Praise. They're a substrate-level addition for any DAO that wants tokenized community capitalization, and they're a precondition for meme-DAO-style launches (the bonding-curve + graduation pattern that Pump.fun popularized).

The original cw-abc work sat in a 2-year-old branch with substantial drift; reviving it means getting the rebase to a clean baseline and treating the audit findings seriously rather than carrying the partial-shape forward.

## Approach

A few load-bearing design decisions, in case reviewers want to push back on them early:

- **Hatcher vesting is inline**, not via per-hatcher cw-vesting instantiation. Each `cw-vesting` instantiate costs ~150–300k gas; a popular hatch with hundreds of hatchers would exceed any per-block gas limit at the Hatch→Open transition. Instead, we track per-hatcher state in `HatcherState` (contributed / minted / already_burned / vesting_started_at / claimed_refund) and compute vested amounts inline against a shared `VestingSchedule`.
- **Factory caller authentication is reverse-handshake**, not an admin allowlist. The factory's intent is for any DAO's voting module to use it during DAO instantiation; an admin allowlist would break that. Instead, when a contract claims to be a DAO's voting module, we ask that DAO whether it agrees, and reject if the round trip doesn't close. This admits any genuinely-related voting module while rejecting impostors. Same pattern applied to `dao-test-custom-factory`.
- **Curve mutability is gated on Closed phase + a 1% continuity check**. The previous unconditional `update_curve` was a clean rug: pause → swap to a curve where `new_curve.supply(reserve) >> current_supply` → buy 1 unit → mint flood → drain. Closing it required both restricting WHEN curves can change and a tolerance check on the (reserve, supply) invariant.
- **Failed-hatch refunds are pro-rata `(reserve + funding)`**, not just reserve. The first round of fixes shipped `AbortHatch` as a transition to Closed (refunds reserve only, owner keeps funding pool). This PR completes M-5 with a `Refunding` sub-state, snapshot-locked pro-rata math, and a permissionless `ClaimRefund` handler. Funding pool is locked from owner withdrawal during Refunding.

## What's in this PR (by commit)

| # | Commit | What |
|---|---|---|
| 1 | `1ec2afe3e` | Rebase cw-abc + cw-curves + dao-abc-factory onto development. 56 commits squash-merged. New workspace members at 2.8.0-alpha.2; `osmosis_tokenfactory` / `cosmwasm_tokenfactory` / `thorchain_tokenfactory` feature pattern adopted to match `dao-voting-token-staked`. |
| 2 | `429a45af2` | **C-1, C-2.** `update_curve` gated on Closed phase + 1% continuity check. Factory reverse-handshake auth (dao-abc-factory + dao-test-custom-factory parity). Factory temp state cleared in reply (L-7). |
| 3 | `7a3fcfced` | **H-1..H-6.** Inline hatcher vesting (`VestingSchedule::{None, Cliff, Linear}` + `HatcherState`). Removed reachable `todo!()` panic. Strict `<` 100% on entry/exit fees. `decimals < 38`. `contribution_limits.min ≤ max`. |
| 4 | `29ba5e12d` | **M-1..M-6.** Priority-queue insert rewrite (`partition_point` instead of broken `binary_search_by`). cw2 migrate guard. Removed dead `HATCHER_ALLOWLIST` map. `hatch_deadline` + `AbortHatch`. Trust-model README. |
| 5 | `84c9bf12` | **L-1..L-7 (except L-5), I-1..I-7.** Bounded query limits, surfaced DAO-query errors, eliminated self-call auth bypass via inline allowlist setup, `decimal()` cast guard, `cube_root` precision raised 9→15. Variant cleanup, exit-fee zeroed on close, naming, doc comments. |
| 6 | `3ba8862f2` | Internal security review + per-finding status table at `audits/2026-05-09-cw-abc-security-review.md`. |
| 7 | `74931ee30` | **L-5.** Curve trait converted to `Result<_, CurveError>`. All `unwrap()`-on-overflow paths replaced with typed `CurveError::{Overflow, DivisionByZero}`. New `ContractError::CurveError` variant. |
| 8 | `744e609fe` | **M-5 full.** `CommonsPhase::Refunding` + `RefundSnapshot` + `ClaimRefund` handler. Pro-rata `(reserve + funding)` refunds, snapshot-locked at AbortHatch time. Buys / sells / withdraw / close blocked in Refunding. |
| 9 | `dc9a678cb` | 30 audit-defense unit tests in `cw-abc/src/audit_tests.rs` covering C-1, H-1 vesting math, H-2..H-6, M-1, M-2, M-5, L-3. |
| 10 | `701dd86b1` | Differential test suite in `cw-curves/src/diff_tests.rs`: SplitMix64-seeded random walks vs f64 reference impls (1k iters per curve per (sd, rd) matrix), round-trip identity, boundary cases. **14/14 cw-curves tests passing.** |
| 11 | `56cbb672e` | Expanded `dao-abc-factory/README.md` (DAO instantiation flow, reverse-handshake auth doc). Audit report status table refreshed: M-5 → Fixed, L-5 → Fixed (20 of 21 fully fixed; L-2 attribute surface partial). |
| 12 | `8c8845bbb` | `cargo fmt --all`. |

## Audit closure summary

20 of 21 findings fully fixed; 1 partial (L-2 attribute surface — explicit Err arm + operator-monitoring note shipped, per-skipped-DAO response attribute deferred since the helper is private and doesn't return attributes today). All Criticals, Highs, and Mediums fully addressed.

| Severity | Count | Status |
|---|---|---|
| Critical (C-1, C-2) | 2 | All fixed |
| High (H-1..H-6) | 6 | All fixed |
| Medium (M-1..M-6) | 6 | All fixed |
| Low (L-1..L-7) | 7 | 6 fixed, 1 partial (L-2) |
| Info (I-1..I-7) | 7 | All fixed |

Per-finding fix-commit refs in [`audits/2026-05-09-cw-abc-security-review.md`](audits/2026-05-09-cw-abc-security-review.md) under "Status as of 2026-05-09".

## Schema-breaking changes (heads up for reviewers)

These would be breaking for any deployed instance, but the branch is pre-mainnet:

- `HATCHERS` map type changed from `Map<&Addr, Uint128>` to `Map<&Addr, HatcherState>`.
- `HatchersResponse` and `Hatcher` query response types updated accordingly.
- `CommonsPhaseConfig` gains a `vesting: VestingSchedule` field.
- `HatchConfig` gains an optional `hatch_deadline: Option<Timestamp>` field.
- `UpdatePhaseConfigMsg` loses the `Closed {}` variant (was a `todo!()` panic).
- `UpdatePhaseConfigMsg::Hatch` gains `hatch_deadline: Option<Option<Timestamp>>`.
- `CommonsPhase` gains a `Refunding` variant.
- `ExecuteMsg` gains `AbortHatch {}` and `ClaimRefund {}`.
- New typed errors: `CurveDriftExceeded`, `InvalidDecimals`, `HatcherTokensNotVested`, `InvalidMigration`, `RefundAlreadyClaimed`, `RefundBurnMismatch`, `CurveError(#[from] cw_curves::CurveError)`.
- Removed: `MismatchedSellAmount`, `Unauthorized` (cw-abc), `UnsupportedFactoryMsg` (dao-abc-factory).

## Verification

Run with the workspace's pinned `nightly-2024-01-08` toolchain. Per-feature gates:

```sh
cargo +nightly-2024-01-08 fmt --all -- --check
cargo +nightly-2024-01-08 clippy --lib -p cw-curves -p cw-abc -p dao-abc-factory \
  --features "cw-abc/cosmwasm_tokenfactory dao-abc-factory/cosmwasm_tokenfactory" \
  -- -D warnings
cargo +nightly-2024-01-08 test -p cw-curves
RUSTFLAGS="-C link-arg=-s" cargo +nightly-2024-01-08 build \
  -p cw-abc -p dao-abc-factory --release --lib --target wasm32-unknown-unknown \
  --features "cw-abc/cosmwasm_tokenfactory dao-abc-factory/cosmwasm_tokenfactory"
```

**Locally green** (all four):
- `cargo fmt --check`: clean
- `cargo clippy --lib -- -D warnings`: clean
- `cargo test -p cw-curves`: **14 passed; 0 failed** (3 happy-path + 3 division-by-zero + 6 differential random walks + 2 round-trip identity + 2 boundary)
- Wasm release builds: `cw_abc.wasm` 16KB, `dao_abc_factory.wasm` 311KB

**Pending CI** (libclang gating; the dev container we built this in didn't have libclang and the pinned nightly doesn't support optional dev-dependencies, so we couldn't gate `osmosis-test-tube` locally):
- `cargo +nightly-2024-01-08 test -p cw-abc -p dao-abc-factory --features cosmwasm_tokenfactory` — 30 unit tests in `cw-abc/src/audit_tests.rs` exercising the full audit-defense matrix.
- `bash scripts/schema.sh` regen for `cw-abc.json` and `dao-abc-factory.json` (the committed JSON is stale).
- `cargo test --features test-tube` for chain-binary integration tests.

## Test plan

- [ ] Workflow: `Basic` (clippy + check) — green expected.
- [ ] Workflow: `Test Tube` — runs the full unit + test-tube suite. **Real CI signal for the new audit_tests.rs surface.**
- [ ] Schema regen: `bash scripts/schema.sh`, then commit the resulting `schema/*.json` deltas if any.
- [ ] Per-feature build matrix: `cosmwasm_tokenfactory`, `osmosis_tokenfactory`, `thorchain_tokenfactory` (the feature unification rejects mixing them).
- [ ] Wasm release artifact sanity: sizes ~16KB and ~311KB respectively.
- [ ] Spot-check: deserialize a malformed `UpdatePhaseConfigMsg::Closed {}` JSON and confirm it now fails (H-2).
- [ ] Spot-check: instantiate with `decimals = 38` and confirm `InvalidDecimals` (H-5).
- [ ] Spot-check: `update_curve` from non-Closed phase rejected with `InvalidPhase` (C-1).

## Known follow-ups (not in this PR)

These are out of scope here but worth a tracking issue:

1. **cw-multi-test integration tests** for through-the-issuer flows: the H-1 vesting matrix via real buy/sell, the C-2 factory reverse-handshake roundtrip with mock voting modules, the M-5 `ClaimRefund` roundtrip with actual mint/burn. The unit tests cover the auth/validation surface; integration tests would cover the value-flow surface.
2. **L-2 response-attribute surface** for skipped DAO queries — the helper is private and doesn't return attributes today; a small refactor would let operators see `try_dao_query_failed` events in logs.
3. **Additional curve types** (S-curve, Taylor series) — referenced as a Future Work bullet in the README.
4. **Testnet deployment** — deferred until Juno's v30 upgrade lands so we deploy against the audited stack.
5. **External audit** by a CosmWasm-experienced security firm. The internal review was a thorough first pass; an external audit before mainnet is recommended.

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Juno AI <juno-ai-dev@users.noreply.github.com>
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
