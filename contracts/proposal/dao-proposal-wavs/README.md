# dao-proposal-wavs

A DAO DAO proposal module that consumes WAVS-attested envelopes.
Reference implementation of [DA0-DA0/dao-contracts#922](https://github.com/DA0-DA0/dao-contracts/issues/922).

> **Status:** v0.1.0-alpha.1 — scaffolding. Path A (mock + trust-based + cw-filter + VetoConfig). See `memory/wavs-proposal-module.md` and `memory/dao-proposal-single-patterns.md` in the parent meta-repo for full design context.

## What it does

Where `dao-proposal-single` lets *humans* propose-and-vote on DAO actions, this module lets a **WAVS service** post an attested envelope that, after replay-check + filter-check + (optional) timelock-with-veto, executes against the DAO treasury.

```
WAVS service                                  Sub-DAO core (treasury)
   │                                                 ▲
   │  signs envelope                                 │ executes msgs
   │  via operator quorum                            │ on Finalize
   ▼                                                 │
ServiceHandlerExecuteMessages::                      │
  WavsHandleSignedEnvelope { envelope, sigs } ───────┘
   │
   ├─► defers signature check to ServiceManager.WavsValidate
   ├─► replay-checks `envelope.eventId` against ATTESTATIONS_SEEN
   ├─► decodes `envelope.payload` as ProposalPayload
   ├─► (optional) runs cw-filter against each msg
   ├─► creates proposal record + applies VetoConfig
   └─► auto-executes (if configured) or queues for explicit Execute
```

## v1 (Path A) design choices

- **Verification deferred.** We use the canonical `wavs-types` ServiceHandler pattern and defer signature verification to the configured service-manager contract. As of 2026-05, cw-middleware's BLS/ECDSA/Mock service-managers all have placeholder verification (`// TODO: real validation logic`). v1 inherits whatever the service-manager does. When Lay3rLabs ships real verification, this module benefits without changes.
- **Replay protection on-contract.** `ATTESTATIONS_SEEN: Map<&[u8; 20], bool>` keyed by `Envelope.eventId`. Submitting the same eventId twice is rejected.
- **cw-filter integration.** Optional `MandateFilterConfig` gates execution: each msg in the payload is passed through `cw-filter.Filter` before queuing. Pass continues; Fail rejects the proposal; Fatal halts.
- **VetoConfig parity.** Same shape as `dao-proposal-single`. Vetoer can kill a proposal during the timelock window between accept and execute.
- **`auto_execute` toggle.** If true, accepted proposals execute immediately (subject to timelock). If false, requires explicit `Execute { proposal_id }` call.

## v2 / v3 (out of scope for v0.1)

- Real BLS quorum verification (when cw-middleware ships, or our fork does).
- ZK-proof-of-policy-compliance via BN254 precompile (when CosmWasm prop #374's upstream PR lands).
- Direct on-chain SGX quote verification.

See `memory/wavs-proposal-module.md` for the full v1/v2/v3 phasing.

## Build

```bash
cd /workspace/contracts/dao-proposal-wavs
cargo build --release --target wasm32-unknown-unknown
# Optional: optimize with wasm-opt
wasm-opt -Oz --signext-lowering --enable-bulk-memory --enable-reference-types \
  target/wasm32-unknown-unknown/release/dao_proposal_wavs.wasm \
  -o ../../artifacts/dao_proposal_wavs.wasm
```

## Naming + upstream coordination

This crate is intended to be PR'd to `DA0-DA0/dao-contracts/contracts/proposal/dao-proposal-wavs` once the design stabilizes. Under that path it would become a workspace member of dao-contracts. Until then it lives standalone in this meta-repo.

When ready to upstream, coordinate with DAO DAO maintainers (Noah Saso et al.) per `memory/jake-relational-norms.md` — bring working code, not idea.

## License

Apache-2.0.
