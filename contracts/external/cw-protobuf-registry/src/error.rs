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

    #[error("No files provided")]
    NoFiles {},

    #[error(
        "File descriptor {file_descriptor_index} missing name in set {file_descriptor_set_index}"
    )]
    FileDescriptorMissingName {
        file_descriptor_index: usize,
        file_descriptor_set_index: usize,
    },

    #[error(
        "File descriptor {file_descriptor_index} (name: {file_descriptor_name}) missing package in set {file_descriptor_set_index}"
    )]
    FileDescriptorMissingPackage {
        file_descriptor_index: usize,
        file_descriptor_name: String,
        file_descriptor_set_index: usize,
    },

    #[error("File descriptor package changed from {file_package} to {new_file_package} for file {file_name}")]
    FileDescriptorPackageChanged {
        file_name: String,
        file_package: String,
        new_file_package: String,
    },

    #[error("Message descriptor {message_descriptor_index} missing name in file {file_name} (package: {file_package})")]
    MessageDescriptorMissingName {
        message_descriptor_index: usize,
        file_name: String,
        file_package: String,
    },

    #[error("Message limit reached before all files were unregistered ({unregistered}/{total}). Increase the limit or unregister fewer files.")]
    MessageLimitReached { unregistered: usize, total: usize },

    #[error("Protobuf message not found: {message}")]
    MessageNotFound { message: String },

    #[error("Internal error: {msg}")]
    InternalError { msg: String },
}
