use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;
use cosmwasm_std::{Addr, Deps, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum PaymentState {
    Paused,
    Active,
    Claimed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Payment {
    pub recipient: String,
    pub amount: Uint128,
    /// A payment must have only one of either a denom or token_address.
    pub denom: Option<String>,
    pub token_address: Option<Addr>,
    /// The amount of time the payment will be locked before being paid.
    pub vesting_time: Expiration,
    /// The number of times this paymount amount will occur. '1' indicates a one-time payment.
    pub num_payments: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CheckedPayment {
    pub recipient: Addr,
    pub amount: Uint128,
    /// A payment must have only one of either a denom or token_address.
    pub denom: Option<String>,
    pub token_address: Option<Addr>,
    /// The amount of time the payment will be locked before being paid.
    pub vesting_time: Expiration,
    pub state: PaymentState,
}

impl Payment {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedPayment, ContractError> {
        let recipient_addr = deps.api.addr_validate(&self.recipient)?;

        // Payment must be only one of native or non-native payment.
        if !(self.denom.is_some() ^ self.token_address.is_some()) {
            return Err(ContractError::ExactlyOnePaymentMethodRequired {});
        }

        // check one of denom and token address
        let checked_payment = CheckedPayment {
            recipient: recipient_addr,
            amount: self.amount,
            denom: self.denom,
            token_address: self.token_address,
            vesting_time: self.vesting_time,
            state: PaymentState::Active,
        };

        Ok(checked_payment)
    }
}

pub const PAYMENT_COUNT: Item<u64> = Item::new("proposal_count");
pub const PAYMENTS: Map<&Addr, CheckedPayment> = Map::new("payments");
pub const ADMIN: Item<Addr> = Item::new("admin_address");
