use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Initial governance token balances must not be empty")]
    InitialBalancesError {},

    #[error("Can not change the contract's token after it has been set")]
    DuplicateToken {},

    #[error("Error instantiating token")]
    TokenInstantiateError {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },
}
