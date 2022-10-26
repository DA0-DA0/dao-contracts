use cosmwasm_std::{StdError, OverflowError, DivideByZeroError, Addr, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    
    #[error("Bad config")]
    BadConfig {},
    
    #[error("Bad config")]
    InsufficientFunds {},

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("Unexpected state")]
    UnexpectedState {},
    
    #[error("Unfunded")]
    Unfunded {},
    
    #[error("Invalid token: received {received}, expected {expected}")]
    InvalidToken {
        received: Addr,
        expected: Addr,
    },

    #[error("Too many claims")]
    TooManyClaims {},

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("The first point of the vest schedule should be a (<timestamp>, 0) pair: Instead the first point is {amount1} at time {time1}. This invariant is required because otherwise the zero-point would be undefined")]
    VestScheduleDoesNotContainInitialZeroPoint {
        amount1: Uint128,
        time1: u64,
    },
    
    #[error("Vest amounts not monotonically increasing over time: Vest amount {amount1} at time {time1} is greater than amount {amount2} at time {time2}")]
    VestScheduleNotMonotonicallyIncreasing {
        amount1: Uint128,
        time1: u64,
        amount2: Uint128,
        time2: u64,
    },

    #[error("Three consecutive vest points defined for the same timestamp, a maximum of two can be used to define a cliff: Vest amount {amount1} at time {time1}, {amount2} at time {time2}, {amount3} at time {time3}")]
    VestScheduleFeaturesMalformedCliff {
        amount1: Uint128,
        time1: u64,
        amount2: Uint128,
        time2: u64,
        amount3: Uint128,
        time3: u64,
    },

    #[error("Only the owner can change the owner")]
    OnlyOwnerCanChangeOwner {},

    #[error("Failure to query the staking contract")]
    QueryFailure {},
}
