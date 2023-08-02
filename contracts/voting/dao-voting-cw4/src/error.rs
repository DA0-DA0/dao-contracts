use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Can not change the contract's token after it has been set")]
    DuplicateGroupContract {},

    #[error("Cannot instantiate a group contract with duplicate initial members")]
    DuplicateMembers {},

    #[error("Error occured whilst instantiating group contract")]
    GroupContractInstantiateError {},

    #[error("Contract only supports queries")]
    NoExecute {},

    #[error("Cannot instantiate or use a group contract with no initial members")]
    NoMembers {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Total weight of the CW4 contract cannot be zero")]
    ZeroTotalWeight {},
}
