use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Payment not found")]
    PaymentNotFound {},

    #[error("Payment is not a native token payment")]
    NotNativePayment {},

    #[error("Payment must either have a native denom or a cw20 token address, but not both.")]
    ExactlyOnePaymentMethodRequired {},

    #[error("Payee ({addr}) not found")]
    PayeeNotFound { addr: Addr },

    #[error("No payment to be claimed.")]
    NothingToClaim {},
}
