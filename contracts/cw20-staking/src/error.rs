use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Cw20Error(#[from] cw20_base::ContractError),
    #[error("Nothing to claim")]
    NothingToClaim {},
}
