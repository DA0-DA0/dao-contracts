use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Staking contract token address does not match provided token address")]
    StakingContractMismatch {},

    #[error("Error instantiating token")]
    TokenInstantiateError {},

    #[error("Can not change the contract's staking contract after it has been set")]
    DuplicateStakingContract {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Can not change the contract's token after it has been set")]
    DuplicateToken {},

    #[error("Initial governance token balances must not be empty")]
    InitialBalancesError {},
}
