use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_denom::UncheckedDenom;
use cw_ownable::cw_ownable_execute;

use crate::{registration::Registration, state::Config};

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the DAO registry. The owner is responsible for approving
    /// and rejecting registrations and receives the fees paid. They also have
    /// admin-level privileges.
    pub owner: String,
    /// The fee amount to register a DAO.
    pub fee_amount: Uint128,
    /// The fee denom to register a DAO.
    pub fee_denom: UncheckedDenom,
    /// How long a registration lasts.
    pub registration_period: Timestamp,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Register or renew with a cw20 token.
    Receive(Cw20ReceiveMsg),
    /// Register with a native token or auto-register by the owner.
    Register {
        /// The name of the registration.
        name: String,
        /// The DAO to register. This can be used by the owner to assign a
        /// registration without a fee or approval step.
        address: Option<String>,
    },
    /// Renew with a native token. Renewal can only occur if the remaining time
    /// is less than the length of a registration. This means you can only renew
    /// once every registration period, to prevent squatting more than one
    /// registration period in advance.
    Renew {},
    /// Approve a pending registration. Only the owner can do this.
    Approve {
        /// The address of the DAO with a pending registration. This is used
        /// instead of the name since several pending registrations may be
        /// trying to register the same name.
        address: String,
    },
    /// Reject a pending registration. Only the owner can do this. The
    /// registration fee is returned to the DAO.
    Reject {
        /// The address of the DAO with a pending registration. This is used
        /// instead of the name since several pending registrations may be
        /// trying to register the same name.
        address: String,
    },
    /// Revoke a registration. Only the owner can do this. The registration fee
    /// is NOT returned to the DAO.
    Revoke {
        /// The name of the registration.
        name: String,
    },
    /// Update the expiration of a registration. Only the owner can do this.
    UpdateExpiration {
        /// The name of the registration.
        name: String,
        /// The new expiration.
        expiration: Timestamp,
    },
    /// Update the config of the registry. Only the owner can do this.
    UpdateConfig {
        /// The new fee amount to register a DAO.
        fee_amount: Option<Uint128>,
        /// The new fee denom to register a DAO.
        fee_denom: Option<UncheckedDenom>,
        /// The new registration period.
        registration_period: Option<Timestamp>,
    },
}

#[cw_serde]
pub enum ReceiveMsg {
    /// Register or renew with a cw20 token.
    Register {
        /// The name of the registration.
        name: String,
    },
    /// Renew with a cw20 token. Renewal can only occur if the remaining time is
    /// less than the length of a registration. This means you can only renew
    /// once every registration period, to prevent squatting more than one
    /// registration period in advance.
    Renew {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the current ownership.
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    /// Returns the configuration of the DAO registry.
    #[returns(Config)]
    Config {},
    /// Returns the active registration for the given address or None if no
    /// active registration exists.
    #[returns(Option<Registration>)]
    Registration {
        /// The address with a registration.
        address: String,
    },
    /// Returns the active registration for the given name or None if no active
    /// registration exists.
    #[returns(Option<Registration>)]
    Resolve {
        /// The name of a registration.
        name: String,
    },
    /// Returns the pending registration for the given address or None if no
    /// pending registration exists.
    #[returns(Option<Registration>)]
    PendingRegistration {
        /// The address with a pending registration.
        address: String,
    },
    /// Returns the most recent registration for the given address or None if
    /// the address has never been registered.
    #[returns(Option<Registration>)]
    MostRecentRegistration {
        /// The address with a registration.
        address: String,
    },
}
