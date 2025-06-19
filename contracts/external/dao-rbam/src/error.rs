use cosmwasm_std::StdError;
use thiserror::Error;

pub use cw_ownable::OwnershipError;
pub use cw_utils::PaymentError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("JSON msg serialization error: {err}")]
    JsonMsgSerialization { err: String },

    #[error("JSON filter error: {err}")]
    JsonFilter { err: String },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("RBAM system is disabled")]
    SystemDisabled {},

    #[error("Role not found with ID: {id}")]
    RoleNotFound { id: u64 },

    #[error("Role is disabled")]
    RoleDisabled {},

    #[error("Authorization not found with ID: {id}")]
    AuthorizationNotFound { id: u64 },

    #[error("Authorization is disabled")]
    AuthorizationDisabled {},

    #[error("Authorization does not belong to the specified role")]
    AuthorizationRoleMismatch {},

    #[error("Address {addr} is not assigned role {role_id}")]
    RoleNotAssigned { addr: String, role_id: u64 },

    #[error("Address {addr} is already assigned role {role_id}")]
    RoleAlreadyAssigned { addr: String, role_id: u64 },

    #[error("Action execution not authorized")]
    ActionNotAuthorized {},
}
