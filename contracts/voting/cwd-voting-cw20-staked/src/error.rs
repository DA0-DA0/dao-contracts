use cosmwasm_std::{StdError, OverflowError};
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

    #[error("Staking contract token address does not match provided token address")]
    StakingContractMismatch {},

    #[error("Can not change the contract's staking contract after it has been set")]
    DuplicateStakingContract {},

    #[error("Active threshold percentage must be greater than 0 and less than 1")]
    InvalidActivePercentage {},

    #[error("Absolute count threshold cannot be greater than the total token supply")]
    InvalidAbsoluteCount {},

    #[error("Vesting contract token address does not match provided token address")]
    VestingContractTokenMismatch {},

    #[error("Vesting contract staking address does not match provided token address")]
    VestingContractStakingMismatch {},

    #[error("Can not change the contract's vesting contract after it has been set")]
    DuplicateVestingContract {},

    #[error("Error instantiating staking contract")]
    StakingInstantiateError {},

    #[error("{0}")]
    Overflow(#[from] OverflowError),
}
