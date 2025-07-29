use cosmwasm_std::{DepsMut, Env, StdResult};

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

pub fn get_module_label(env: &Env, suffix: &str) -> String {
    let last6 = env.contract.address.to_string().chars().rev().take(6).collect::<String>();
    format!("rbam-{}-{}", last6, suffix)
}
