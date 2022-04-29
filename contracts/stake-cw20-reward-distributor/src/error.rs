use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("No Pending Payments")]
    NoPendingPayments {},

    #[error("Out of Funds")]
    OutOfFunds {},

    #[error("Invalid Cw20")]
    InvalidCw20 {},

    #[error("Invalid Staking Contract")]
    InvalidStakingContract {},
}
