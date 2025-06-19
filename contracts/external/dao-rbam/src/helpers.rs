use cosmwasm_std::{DepsMut, StdResult};

use crate::{
    state::{ENABLED, NEXT_ID},
    ContractError,
};

/// Ensure the RBAM system is enabled.
pub fn ensure_enabled(deps: DepsMut) -> Result<(), ContractError> {
    if !ENABLED.load(deps.storage)? {
        return Err(ContractError::SystemDisabled {});
    }
    Ok(())
}

/// Gets the next ID and increments state by first incrementing in state and
/// then decrementing the updated return value.
pub fn get_next_id(deps: DepsMut) -> StdResult<u64> {
    NEXT_ID
        // Increment the ID in state
        .update(deps.storage, |id| Ok(id + 1))
        // Decrement the new ID to get the previous ID
        .map(|id| id - 1)
}
