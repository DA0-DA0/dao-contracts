use cosmwasm_std::StdError;
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

    #[error("Error instantiating staking contract")]
    StakingInstantiateError {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Staking contract token address does not match provided token address")]
    StakingContractMismatch {},

    #[error("Can not change the contract's staking contract after it has been set")]
    DuplicateStakingContract {},

    #[error("Active threshold percentage must be greater than 0 and less than 1")]
    InvalidActivePercentage {},

    #[error("Active threshold count must be greater than zero")]
    ZeroActiveCount {},

    #[error("Absolute count threshold cannot be greater than the total token supply")]
    InvalidAbsoluteCount {},
}
