//! Envelope verification flow — Path A: defer signature check to the configured service-manager.
//!
//! Once `cw-middleware`'s service-manager ships real verification (currently placeholder per
//! the source comments), this module benefits without code changes — we already use the
//! canonical `WavsValidate` query.

use cosmwasm_std::{Deps, StdResult};

use crate::error::ContractError;
use crate::wavs_compat::{
    ServiceManagerQueryMessages, WavsEnvelope, WavsSignatureData, WavsValidateResult,
};

/// Calls `service_manager.WavsValidate { envelope, signature_data }`.
/// Returns Ok(()) if the manager accepts; ContractError::SignatureInvalid otherwise.
pub fn validate_envelope(
    deps: Deps,
    service_manager: &str,
    envelope: &WavsEnvelope,
    signature_data: &WavsSignatureData,
) -> Result<(), ContractError> {
    let result: WavsValidateResult = deps.querier.query_wasm_smart(
        service_manager,
        &ServiceManagerQueryMessages::WavsValidate {
            envelope: envelope.clone(),
            signature_data: signature_data.clone(),
        },
    )?;

    match result {
        WavsValidateResult::Ok => Ok(()),
        WavsValidateResult::Err(e) => Err(ContractError::SignatureInvalid { reason: e }),
    }
}

/// SHA256 of arbitrary bytes — used for replay-tracking by event_id (and by tests for binding
/// payload digests).
pub fn sha256(bytes: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

/// Convert `event_id` bytes to a lowercase hex string for storage / display.
pub fn event_id_hex(event_id: &[u8]) -> String {
    event_id.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Helper: load the `Empty`-msg result type expected by ServiceHandler.
pub fn _unit_ok() -> StdResult<()> {
    Ok(())
}
