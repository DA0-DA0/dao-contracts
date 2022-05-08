use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds sent")]
    InsufficientFunds {},

    #[error("This name is reserved for later user")]
    NameReserved {},

    #[error("This name is not reserved for later use")]
    NameNotReserved {},

    #[error("This name is already taken by another DAO")]
    NameAlreadyTaken {},

    #[error("You already registered a name with this DAO")]
    AlreadyRegisteredName {},

    #[error("Invalid payment amount, amount cannot be zero")]
    InvalidPaymentAmount {},

    #[error("This name is not registered to a DAO")]
    NameNotRegistered {},
}
