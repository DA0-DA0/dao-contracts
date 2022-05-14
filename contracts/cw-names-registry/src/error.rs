use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Incorrect payment amount")]
    IncorrectPaymentAmount {},

    #[error("This name is reserved for later use")]
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

    #[error("Invalid CW20, this address is not a CW20")]
    InvalidCw20 {},

    #[error("This CW20's address does not match the configured CW20 payment address")]
    UnrecognisedCw20 {},

    #[error("This token's denom does not match the configured token's denom")]
    UnrecognisedNativeToken {},

    #[error("Invalid payment")]
    InvalidPayment {},
}
