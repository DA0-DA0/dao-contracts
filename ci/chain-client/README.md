# Native integration-chain client

This deliberately limited facade serves `ci/integration-tests` and
`ci/bootstrap-env`. It replaces their cosm-orc/cosm-tome dependency without
changing contract messages, Test Tube, the optimizer, or the pinned Juno fixture.
It is not a general-purpose or production wallet client.

## Configuration and compatibility

Existing `chain_cfg` and `contract_deploy_info` YAML keys and configuration paths
are retained, including the generated `ci/configs/cosm-orc/local.yaml` file.
`CONFIG`, `GAS_OUT_DIR`, and `SKIP_CONTRACT_STORE` keep their existing meanings in
the integration harness. The historical `cosm-orc` directory/variable names do
not mean that the old client remains in use.

Set `chain_cfg.rpc_endpoint`, for example `http://127.0.0.1:26657` for the local
fixture. `grpc_endpoint` is preserved when serializing configuration but is not
used. A grpc-only configuration now fails with an explicit missing-RPC error;
an RPC endpoint cannot safely be inferred from a gRPC URL. For custom networks,
supply an endpoint for the configured chain yourself. This migration does not
retarget the historical `uni-5` configuration or its stored code IDs.

Before signing, the client verifies the configured chain ID and requires a
CometBFT **0.37 or 0.38** node, rejecting other versions. Its compatibility
pilots target the pinned Juno v28.0.2 fixture (SDK 0.50.11, CometBFT 0.38.17,
wasmd 0.54.0, wasmvm 2.2.2). Compatibility with other node versions is not
implied by the upstream RPC crate's additional dialects.

## Transaction policy

- One 30-second operation budget covers status, account lookup, simulation,
  signing, network requests and the committed response, **per transaction**, not
  per artifact batch. Requests and the enclosing operation are bounded.
- Sign once, calculate the local SHA256 hash, and submit once through
  `broadcast_tx_commit`. The 0.37 request encoding is identical on 0.38 and the
  response type aliases both field spellings, which the pinned fixture's full
  storage run verifies. HTTP redirects are disabled.
- Require CheckTx success, the matching hash, positive committed height, and
  execution success. CheckTx alone is never an execution receipt. The maintained
  RPC decoder requires Comet's canonical uppercase hash encoding; noncanonical
  lowercase or malformed hashes are rejected, not normalized.
- Preserve committed log, data, events, gas wanted and gas used. The RPC library's
  execution-data bytes retain base64 text, which is decoded explicitly. **Event
  attributes are already plain strings in 0.37/0.38 and must not be
  base64-decoded.**
- Resolve code IDs and instantiated addresses from the single matching top-level
  protobuf response, not an arbitrary nested instantiation event. Verify a store
  response's checksum before updating the contract registry.
- Network/decode failures are distinct from simulation, CheckTx and execution
  rejections. Timeout or broadcast transport failure can have an **unknown
  outcome**; the local hash is retained where available. Never automatically
  retry, re-sign or resubmit. The harness caches setup failures, including panics,
  so subsequent tests do not implicitly repeat partially completed storage.

The prior uncompressed Wasm bytes, derivation path, empty BIP39 passphrase,
bech32 prefix, simulation envelope, transaction memo and instantiate label are
preserved. Legacy memo/label strings remain intentionally: changing them changes
transaction size and gas. The fee calculation remains
`ceil(simulated_gas * gas_adjustment)`, then `ceil(gas_limit * gas_price)` (1.5 and
0.1 in the fixture). There is no automatic fee/gas increase. Gas report keys and
`gas_wanted`, `gas_used`, `file_name`, `line_number` fields remain compatible.

## Dependency boundary

The crate and all dependencies are gated out of wasm32. It has no `cdylib` target
and adds no optimizer contract artifact. Contract-target dependency/feature trees
must remain unchanged; optimized artifact checksums and full CI still need
verification after changes.

Core published versions are cosmrs 0.22.0, cosmos-sdk-proto 0.27.0 and
tendermint-rpc 0.40.4. The proto crate declares Rust 1.75; verification uses the
repository's existing `nightly-2024-01-08` (Rust 1.77), not a newer compiler.
The committed lockfile deliberately constrains native transitive versions,
including toml 0.8.12, toml_edit 0.22.12, indexmap 2.2.6, uuid 1.8.0 and peg 0.8.5.
Unconstrained resolution can select packages requiring a newer compiler.

The native TLS path introduces ring 0.17.14 and requires changing the shared
native build dependency **cc 1.1.6 to 1.2.8**. This is a real dependency change,
not a claim of zero dependency impact. cc is absent from the contract Wasm graph;
it also appears in a Haiku-only chrono build path in an all-platform graph.
Older ring versions were not chosen simply to avoid that delta:
[RUSTSEC-2025-0009](https://rustsec.org/advisories/RUSTSEC-2025-0009) is fixed in
0.17.12+, whose build dependency already requires cc 1.2.8+. Existing older
transitive versions still needed elsewhere must not be removed indiscriminately.
Published dependency license metadata is permissive (Apache/MIT/BSD/ISC/CC0,
including combination licenses); retain the upstream notices and lock checksums.

## Verification

From the repository root:

```sh
rustup run nightly-2024-01-08 cargo test --locked -p dao-chain-client
rustup run nightly-2024-01-08 cargo clippy --locked -p dao-chain-client \
  -p integration-tests -p bootstrap-env --all-targets -- -D warnings
```

The ordinary tests use loopback mock RPC servers and exercise actual signing and
client calls, malformed/rejected responses, deadlines, one-submission behavior,
plain-string events, gas/config/registry compatibility, and cached setup failure.
The opt-in pilot is excluded from ordinary unit runs because it changes a chain.
Run it only against an explicitly created **throwaway local pinned node**, using
already optimized artifacts and their checksum manifest:

```sh
PILOT_RPC=http://127.0.0.1:26657 PILOT_ARTIFACT_DIR="$PWD/artifacts" \
  rustup run nightly-2024-01-08 cargo test --locked -p dao-chain-client \
  rpc::pinned_juno::pinned_juno -- --ignored --exact --nocapture --test-threads=1
```

The pilots check account/bank/validator queries, block polling, an actual
checksum-verified upload, instantiate/execute/query and rejection handling.
`pinned_juno_store_all` additionally stores every artifact in the directory —
including the largest one, which only fits the RPC body limit the fixture
bootstrap raises — and requires the complete registry at the end. To
exercise committed execution failure, its **test-only** fault injection submits
an unauthorized execute without simulation; production `transact` always
simulates. No limits are raised, and no failed submission is retried.

Neither mock tests nor this small pilot replace the complete original Integration
and Test Tube suites or Basic CI on the same final head. Full integration storage
must include every produced artifact, including RBAM/filter and named variants.
