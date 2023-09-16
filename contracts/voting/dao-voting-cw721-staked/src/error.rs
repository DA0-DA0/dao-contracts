use cosmwasm_std::{Addr, StdError};
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Can not stake that which has already been staked")]
    AlreadyStaked {},

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

    #[error("Active threshold count is greater than supply")]
    InvalidActiveCount {},

    #[error("Active threshold percentage must be greater than 0 and less than 1")]
    InvalidActivePercentage {},

    #[error("Invalid token. Got ({received}), expected ({expected})")]
    InvalidToken { received: Addr, expected: Addr },

    #[error("Error instantiating NFT contract")]
    NftInstantiateError {},

    #[error("New NFT contract must be instantiated with at least one NFT")]
    NoInitialNfts {},

    #[error("Factory contract did not implment the required NftFactoryCallback interface")]
    NoFactoryCallback {},

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Only the owner of this contract my execute this message")]
    NotOwner {},

    #[error("Can not unstake that which you have not staked (unstaking {token_id})")]
    NotStaked { token_id: String },

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Factory message must serialize to WasmMsg::Execute")]
    UnsupportedFactoryMsg {},

    #[error("Active threshold count must be greater than zero")]
    ZeroActiveCount {},

    #[error("Can't unstake zero NFTs.")]
    ZeroUnstake {},
}
