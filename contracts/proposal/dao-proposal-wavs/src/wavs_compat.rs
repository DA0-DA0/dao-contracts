//! Wire-format types mirroring `wavs-types::contracts::cosmwasm::*` for the small surface we need.
//!
//! We avoid importing the upstream `wavs-types` crate as a direct dep because its alloy
//! sub-deps cause version-resolution conflicts in this workspace. Re-declaring the cw_serde
//! shapes here keeps us decoupled and lets the contract compile against vanilla cosmwasm.
//!
//! **Confirmed against** `wavs/packages/types/src/contracts/cosmwasm/service_handler.rs` and
//! `wavs/contracts/solidity/interfaces/IWavsServiceHandler.sol` 2026-05-08. If wavs-types' wire
//! format changes upstream, update this file in lock-step.
//!
//! The Solidity types being wrapped:
//! ```solidity
//! struct Envelope { bytes20 eventId; bytes12 ordering; bytes payload; }
//! struct SignatureData { address[] signers; bytes[] signatures; uint32 referenceBlock; }
//! ```

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

/// `WavsEnvelope` is an ABI-encoded `Envelope { eventId, ordering, payload }` wrapped as
/// opaque bytes. We don't need to decode the Solidity layout on-chain; we extract the
/// `eventId` (first 20 bytes) and `payload` via index slicing in the verify path.
#[cw_serde]
pub struct WavsEnvelope(pub Binary);

impl WavsEnvelope {
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Extract the 20-byte eventId. Returns an error if the envelope is shorter than 20 bytes.
    ///
    /// **v0.2 wire format note.** Canonical Solidity ABI for `struct Envelope { bytes20 eventId;
    /// bytes12 ordering; bytes payload; }` would dynamically encode the fields. We adopt a
    /// simpler fixed-prefix layout for v0.2:
    ///   - bytes 0..20  : eventId
    ///   - bytes 20..32 : ordering (reserved; ignored in v0.2)
    ///   - bytes 32..   : payload (JSON-serialized ProposalPayload)
    ///
    /// WAVS services targeting this contract must produce envelopes in this packed format.
    /// v0.3+ may add a Solidity-ABI-decoded path behind a feature flag.
    pub fn event_id(&self) -> Result<&[u8], &'static str> {
        if self.0.len() < 20 {
            return Err("envelope shorter than 20 bytes — invalid wire format");
        }
        Ok(&self.0[..20])
    }

    /// Extract the payload bytes (everything after the 32-byte fixed prefix).
    /// Returns an error if the envelope is shorter than 32 bytes.
    pub fn payload(&self) -> Result<&[u8], &'static str> {
        if self.0.len() < 32 {
            return Err("envelope shorter than 32 bytes — no room for payload");
        }
        Ok(&self.0[32..])
    }
}

/// Mirror of `WavsSignatureData`. `signers` are bech32 addresses; `signatures` are opaque bytes.
#[cw_serde]
pub struct WavsSignatureData {
    pub signers: Vec<String>,
    pub signatures: Vec<Binary>,
    pub reference_block: u32,
}

/// The execute-side messages the WAVS service emits to a service-handler contract.
#[cw_serde]
pub enum ServiceHandlerExecuteMessages {
    WavsHandleSignedEnvelope {
        envelope: WavsEnvelope,
        signature_data: WavsSignatureData,
    },
}

/// The query-side message a service-handler exposes for "where's my service-manager".
#[cw_serde]
pub enum ServiceHandlerQueryMessages {
    WavsServiceManager {},
}

/// The query-side message we send to the configured service-manager to validate an envelope.
#[cw_serde]
pub enum ServiceManagerQueryMessages {
    WavsValidate {
        envelope: WavsEnvelope,
        signature_data: WavsSignatureData,
    },
}

/// Mirror of `wavs_types::WavsValidateResult`. The upstream `Err` variant carries a typed
/// `WavsValidateError`; for v0.1 we reduce it to a string and reify the typed error in v0.2
/// when we need richer error handling.
#[cw_serde]
pub enum WavsValidateResult {
    Ok,
    Err(String),
}
