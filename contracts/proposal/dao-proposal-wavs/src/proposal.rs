use cosmwasm_std::Storage;

use crate::error::ContractError;
use crate::state::PROPOSAL_COUNT;

/// Increment-and-load the proposal counter. Returns the new id.
pub fn advance_proposal_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id: u64 = PROPOSAL_COUNT
        .may_load(storage)?
        .unwrap_or_default()
        .checked_add(1)
        .ok_or_else(|| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(
                "proposal count overflow",
            ))
        })?;
    PROPOSAL_COUNT.save(storage, &id)?;
    Ok(id)
}
