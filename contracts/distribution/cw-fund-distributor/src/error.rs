use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Zero voting power")]
    ZeroVotingPower {},

    #[error("Zero funds")]
    ZeroFunds {},

    #[error("Cannot claim funds during the funding period")]
    ClaimDuringFundingPeriod {},

    #[error("Cannot fund the contract during the claim period")]
    FundDuringClaimingPeriod {},

    #[error("List of specified tokens to claim is empty")]
    EmptyClaim {},

    #[error("{0}")]
    OverflowErr(#[from] OverflowError),
}
