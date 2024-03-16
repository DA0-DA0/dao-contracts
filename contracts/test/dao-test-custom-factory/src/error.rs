use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};
use dao_voting::threshold::ActiveThresholdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    ActiveThresholdError(#[from] ActiveThresholdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error(transparent)]
    PaymentError(#[from] PaymentError),

    #[error("New NFT contract must be instantiated with at least one NFT")]
    NoInitialNfts {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Factory message must serialize to WasmMsg::Execute")]
    UnsupportedFactoryMsg {},
}
