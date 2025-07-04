use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
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
    ParseReply(#[from] ParseReplyError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("JSON serialization error: {err}")]
    JsonSerialization { err: String },

    #[error("Unknown reply ID: {id}")]
    UnknownReplyID { id: u64 },

    #[error("Missing protobuf registry")]
    MissingProtobufRegistry {},
}

impl From<ContractError> for String {
    fn from(err: ContractError) -> Self {
        err.to_string()
    }
}
