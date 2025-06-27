use cosmwasm_std::StdError;
use prost_reflect::prost::DecodeError;
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

    #[error(transparent)]
    Prost(#[from] DecodeError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("JSON serialization error: {err}")]
    JsonSerialization { err: String },

    #[error("Invalid filter: {err}")]
    FilterInvalid { err: String },

    #[error("Message not allowed by filter: {err}")]
    MsgNotAllowedByFilter { err: String },

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

    #[error("Action not found with ID: {id}")]
    ActionNotFound { id: u64 },

    #[error("No authorization filter set")]
    NoAuthorizationFilterSet {},

    #[error("No actions to execute")]
    NoActions {},

    #[error("No roles provided")]
    NoRoles {},

    #[error("Pagination limit reached")]
    LimitReached {},

    #[error("No more authorizations to check")]
    NoMoreAuthorizations {},

    #[error("No files provided")]
    NoFiles {},

    #[error(
        "File descriptor {file_descriptor_index} missing name in set {file_descriptor_set_index}"
    )]
    FileDescriptorMissingName {
        file_descriptor_index: usize,
        file_descriptor_set_index: usize,
    },

    #[error("Message descriptor {message_descriptor_index} missing name in file {file_name}")]
    MessageDescriptorMissingName {
        message_descriptor_index: usize,
        file_name: String,
    },

    #[error("Message limit reached before all files were unregistered ({unregistered}/{total}). Increase the limit or unregister fewer files.")]
    ProtobufMessageLimitReached { unregistered: usize, total: usize },
}

impl From<ContractError> for String {
    fn from(err: ContractError) -> Self {
        err.to_string()
    }
}
