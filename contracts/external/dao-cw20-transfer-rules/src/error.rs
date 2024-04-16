use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("invalid DAO: {error}")]
    InvalidDao { error: StdError },

    #[error("Unauthorized: sender not allowed to send tokens")]
    UnauthorizedSender {},

    #[error("Unauthorized: recipient not allowed to receive tokens")]
    UnauthorizedRecipient {},
}
