//! Wire-format types mirroring `wavs-types::contracts::cosmwasm::*` for the small surface we need.
//!
//! We avoid importing `wavs-types` as a direct dep because (a) its alloy sub-deps cause
//! version-resolution conflicts in this workspace and (b) `wavs-types` requires Rust 1.86+
//! while dao-contracts' CI pins `nightly-2024-01-08` (~1.77). Re-declaring the cw_serde
//! shapes here keeps us decoupled and lets the contract compile against the project's
//! vanilla cosmwasm toolchain.
//!
//! **Confirmed against** `wavs-types` 2.0.0-rc.8 and
//! `Lay3rLabs/cw-middleware` ship-v0.3.0 (PR #77 @ 289277a). The wire formats match
//! exactly so envelopes signed by a real WAVS service round-trip through cw-middleware's
//! `WavsValidate` query without change. If wavs-types' wire format bumps upstream, update
//! this file in lock-step.
//!
//! The Solidity source types being wrapped:
//! ```solidity
//! struct Envelope { bytes20 eventId; bytes12 ordering; bytes payload; }
//! struct SignatureData { address[] signers; bytes[] signatures; uint32 referenceBlock; }
//! ```

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, HexBinary, Uint256};

/// `WavsEnvelope` is the ABI-encoded `Envelope { eventId, ordering, payload }` wrapped as
/// opaque bytes. JSON-serializes as base64, matching `wavs-types`' `WavsEnvelope(Binary)`
/// newtype byte-for-byte. The service-manager hashes these exact bytes for verification
/// (ECDSA: `eip191(keccak256(envelope))`, BLS: hash-to-curve over DST), so the off-chain
/// signer's view of the envelope and our on-chain `WavsValidate` argument must be
/// bit-identical.
#[cw_serde]
pub struct WavsEnvelope(pub Binary);

impl WavsEnvelope {
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Extract the 20-byte eventId from the canonical ABI head slot 0
    /// (`bytes20 eventId` is right-padded into the first 32-byte slot; the eventId itself
    /// occupies the leading 20 bytes).
    pub fn event_id(&self) -> Result<&[u8], &'static str> {
        if self.0.len() < 96 {
            return Err("envelope shorter than 96 bytes — invalid ABI head");
        }
        Ok(&self.0[..20])
    }

    /// Extract the dynamic `payload` bytes from the canonical Solidity-ABI encoding of
    /// `Envelope { bytes20 eventId; bytes12 ordering; bytes payload; }`.
    ///
    /// Head layout:
    ///   - slot 0 (bytes 0..32):  eventId, right-padded
    ///   - slot 1 (bytes 32..64): ordering, right-padded
    ///   - slot 2 (bytes 64..96): offset pointer to payload tail (uint256 big-endian)
    ///
    /// Tail layout (starting at `offset`):
    ///   - 32 bytes: payload length (uint256 big-endian)
    ///   - N bytes:  payload data, then right-padded to a 32-byte multiple
    pub fn payload(&self) -> Result<&[u8], &'static str> {
        let bytes = self.as_slice();
        if bytes.len() < 96 {
            return Err("envelope shorter than 96 bytes — invalid ABI head");
        }

        let offset = read_u64_be(&bytes[64..96])? as usize;
        let length_end = offset.checked_add(32).ok_or("payload offset+32 overflow")?;
        if length_end > bytes.len() {
            return Err("payload length slot out of range");
        }
        let length = read_u64_be(&bytes[offset..length_end])? as usize;
        let data_end = length_end
            .checked_add(length)
            .ok_or("payload data end overflow")?;
        if data_end > bytes.len() {
            return Err("payload data extends beyond envelope");
        }
        Ok(&bytes[length_end..data_end])
    }
}

/// Read a uint256 (32 bytes big-endian) clamped to u64. Returns an error if the upper
/// 24 bytes are non-zero (i.e. the value exceeds u64). Sufficient for ABI offset/length
/// reads on any sensibly-sized envelope.
fn read_u64_be(slot: &[u8]) -> Result<u64, &'static str> {
    if slot.len() != 32 {
        return Err("uint256 slot not 32 bytes");
    }
    if slot[..24].iter().any(|&b| b != 0) {
        return Err("uint256 value exceeds u64");
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&slot[24..32]);
    Ok(u64::from_be_bytes(buf))
}

/// Mirror of `wavs-types::WavsSignatureData`.
///
/// `signers` carries 20-byte signing-key identifiers as `0x`-prefixed lowercase hex
/// strings — the JSON shape `layer_climb_address::EvmAddr` produces. We keep `String`
/// here to avoid pulling in `layer-climb-address`; cw-middleware deserializes either
/// way since `EvmAddr` derives `Deserialize` from a hex string.
///
/// `signatures` carries opaque per-signer signatures as `HexBinary` — for ECDSA
/// 65-byte secp256k1 (r||s||v with v ∈ {0,1,27,28}), for BLS one aggregate G2
/// signature (96-byte compressed). JSON shape is the lower-case hex string without
/// 0x prefix, matching `wavs-types`.
#[cw_serde]
pub struct WavsSignatureData {
    pub signers: Vec<String>,
    pub signatures: Vec<HexBinary>,
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

/// Mirror of `wavs-types::WavsValidateError`. Field shapes match byte-for-byte so JSON
/// returned by a real cw-middleware service-manager round-trips into this enum.
#[cw_serde]
pub enum WavsValidateError {
    InvalidSignatureLength,
    InvalidSignatureOrder,
    InvalidSignature(String),
    InsufficientQuorumZero,
    InsufficientQuorum {
        signer_weight: Uint256,
        threshold_weight: Uint256,
        total_weight: Uint256,
    },
    InvalidQuorumParameters,
}

impl core::fmt::Display for WavsValidateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSignatureLength => write!(f, "invalid signature length"),
            Self::InvalidSignatureOrder => write!(f, "invalid signature order"),
            Self::InvalidSignature(s) => write!(f, "invalid signature: {s}"),
            Self::InsufficientQuorumZero => write!(f, "insufficient quorum: zero signers"),
            Self::InsufficientQuorum {
                signer_weight,
                threshold_weight,
                total_weight,
            } => write!(
                f,
                "insufficient quorum: signer weight {signer_weight} below threshold {threshold_weight} of total {total_weight}",
            ),
            Self::InvalidQuorumParameters => write!(f, "invalid quorum parameters"),
        }
    }
}

/// Mirror of `wavs-types::WavsValidateResult`.
#[cw_serde]
pub enum WavsValidateResult {
    Ok,
    Err(WavsValidateError),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a canonical Solidity-ABI encoding of `Envelope { eventId, ordering, payload }`.
    /// Mirrors `alloy_sol_types::SolValue::abi_encode` for the struct.
    fn abi_encode_envelope(event_id: [u8; 20], ordering: [u8; 12], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(96 + 32 + payload.len().div_ceil(32) * 32);
        // Slot 0: eventId, right-padded
        out.extend_from_slice(&event_id);
        out.extend_from_slice(&[0u8; 12]);
        // Slot 1: ordering, right-padded
        out.extend_from_slice(&ordering);
        out.extend_from_slice(&[0u8; 20]);
        // Slot 2: offset to payload tail = 96 (head size)
        let mut slot = [0u8; 32];
        slot[24..32].copy_from_slice(&96u64.to_be_bytes());
        out.extend_from_slice(&slot);
        // Tail: length prefix
        let mut slot = [0u8; 32];
        slot[24..32].copy_from_slice(&(payload.len() as u64).to_be_bytes());
        out.extend_from_slice(&slot);
        // Tail: payload data, padded to 32-byte multiple
        out.extend_from_slice(payload);
        let pad = (32 - (payload.len() % 32)) % 32;
        if pad > 0 {
            out.extend(std::iter::repeat(0u8).take(pad));
        }
        out
    }

    #[test]
    fn envelope_extracts_event_id_and_payload() {
        let event_id = [42u8; 20];
        let payload = b"hello world";
        let bytes = abi_encode_envelope(event_id, [0u8; 12], payload);
        let env = WavsEnvelope(Binary::from(bytes));
        assert_eq!(env.event_id().unwrap(), &event_id[..]);
        assert_eq!(env.payload().unwrap(), &payload[..]);
    }

    #[test]
    fn envelope_rejects_short_head() {
        let env = WavsEnvelope(Binary::from(vec![0u8; 95]));
        assert!(env.event_id().is_err());
        assert!(env.payload().is_err());
    }

    #[test]
    fn envelope_rejects_offset_out_of_range() {
        // 96-byte head, offset slot points past end-of-envelope.
        let mut bytes = vec![0u8; 96];
        let mut slot = [0u8; 32];
        slot[24..32].copy_from_slice(&999u64.to_be_bytes());
        bytes[64..96].copy_from_slice(&slot);
        let env = WavsEnvelope(Binary::from(bytes));
        assert!(env.payload().is_err());
    }

    #[test]
    fn empty_payload_is_valid() {
        let bytes = abi_encode_envelope([1u8; 20], [0u8; 12], &[]);
        let env = WavsEnvelope(Binary::from(bytes));
        assert_eq!(env.payload().unwrap(), &[] as &[u8]);
    }
}
