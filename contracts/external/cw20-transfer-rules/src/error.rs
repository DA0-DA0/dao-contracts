use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("Unauthorized: cannot transfer to an address which is not part of the DAO or on the allowlist.")]
    Unauthorized {},
}
