use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Cw20(#[from] cw20_base::ContractError),

    #[error(transparent)]
    Ownable(#[from] cw_ownable::OwnershipError),

    #[error(transparent)]
    HookError(#[from] cw_controllers::HookError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown ID: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Hook errored: {error}")]
    HookErrored { error: String },

    #[error("Invalid migration. Expected contract {expected}, got {actual}")]
    InvalidMigration { expected: String, actual: String },
}
