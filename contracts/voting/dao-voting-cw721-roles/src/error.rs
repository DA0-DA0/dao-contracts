use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Error instantiating cw721-roles contract")]
    NftInstantiateError {},

    #[error("This contract only supports queries")]
    NoExecute {},

    #[error("New cw721-roles contract must be instantiated with at least one NFT")]
    NoInitialNfts {},

    #[error("Only the owner of this contract my execute this message")]
    NotOwner {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },
}
