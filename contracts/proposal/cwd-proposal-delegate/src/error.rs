use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Delegation not found")]
    DelegationNotFound {},

    #[error("Delegation is irrevocable")]
    DelegationIrrevocable {},

    #[error("Delegation is expired")]
    DelegationExpired {},
}
