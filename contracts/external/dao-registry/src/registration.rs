use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, StdResult, Timestamp, WasmMsg,
};
use cw_denom::CheckedDenom;

use crate::{
    state::{Config, CONFIG, NAMES, PENDING_REGISTRATIONS, REGISTRATIONS},
    ContractError,
};

#[cw_serde]
pub enum RegistrationStatus {
    /// The registration is pending approval.
    Pending {
        /// The config that was used to register. This is necessary in case the
        /// fee or registration period change after a request is submitted.
        config: Config,
    },
    /// The registration was approved.
    Approved,
    /// The registration is rejected.
    Rejected,
    /// The registration is revoked.
    Revoked,
}

#[cw_serde]
pub struct Registration {
    /// The status of the registration.
    pub status: RegistrationStatus,
    /// The address of the DAO.
    pub address: Addr,
    /// The unique name of the DAO.
    pub name: String,
    /// When the registration expires.
    pub expiration: Timestamp,
}

impl Registration {
    pub fn new(address: Addr, name: String, config: Config) -> Self {
        Self {
            status: RegistrationStatus::Pending { config },
            address,
            name,
            expiration: Timestamp::from_nanos(0),
        }
    }

    // Approve pending registration.
    pub fn approve(&mut self, env: &Env, deps: DepsMut) -> Result<Self, ContractError> {
        if !self.is_pending() {
            return Err(ContractError::NoPendingRegistrationFound);
        }

        PENDING_REGISTRATIONS.remove(deps.storage, self.address.clone());
        NAMES.save(deps.storage, self.name.clone(), &self.address)?;

        let config = CONFIG.load(deps.storage)?;
        let registration =
            REGISTRATIONS.update(deps.storage, self.address.clone(), |registration| {
                if let Some(mut registration) = registration {
                    registration.status = RegistrationStatus::Approved;
                    registration.expiration = env
                        .block
                        .time
                        .plus_nanos(config.registration_period.nanos());

                    Ok(registration)
                } else {
                    Err(ContractError::NoPendingRegistrationFound)
                }
            })?;

        Ok(registration)
    }

    // Reject pending registration.
    pub fn reject(&mut self, env: &Env, deps: DepsMut) -> Result<Self, ContractError> {
        if !self.is_pending() {
            return Err(ContractError::NoPendingRegistrationFound);
        }

        PENDING_REGISTRATIONS.remove(deps.storage, self.address.clone());

        let registration =
            REGISTRATIONS.update(deps.storage, self.address.clone(), |registration| {
                if let Some(mut registration) = registration {
                    registration.status = RegistrationStatus::Rejected;
                    registration.expiration = env.block.time;

                    Ok(registration)
                } else {
                    Err(ContractError::NoPendingRegistrationFound)
                }
            })?;

        Ok(registration)
    }

    // Revoke active registration.
    pub fn revoke(&mut self, env: &Env, deps: DepsMut) -> Result<Self, ContractError> {
        if !self.is_active(env) {
            return Err(ContractError::NoRegistrationFound);
        }

        let registration =
            REGISTRATIONS.update(deps.storage, self.address.clone(), |registration| {
                if let Some(mut registration) = registration {
                    registration.status = RegistrationStatus::Revoked;
                    registration.expiration = env.block.time;

                    Ok(registration)
                } else {
                    Err(ContractError::NoPendingRegistrationFound)
                }
            })?;

        Ok(registration)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.status, RegistrationStatus::Pending { .. })
    }

    pub fn is_active(&self, env: &Env) -> bool {
        match self.status {
            // If approved, active if not yet expired.
            RegistrationStatus::Approved => !self.is_expired(env),
            _ => false,
        }
    }

    pub fn is_expired(&self, env: &Env) -> bool {
        self.expiration <= env.block.time
    }

    // Renewable if active and within one registration period of expiration. If
    // not, it has already been renewed for the next period.
    pub fn is_renewable(&self, env: &Env, deps: &DepsMut) -> Result<bool, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        Ok(self.is_active(env)
            && self.expiration
                <= env
                    .block
                    .time
                    .plus_nanos(config.registration_period.nanos()))
    }

    // Get the cosmos msg to transfer the fee. When pending, the stored config
    // is used. When renewing after approved, the current config is used.
    pub fn get_transfer_msg(
        &self,
        to: &Addr,
        config: Option<Config>,
    ) -> Result<CosmosMsg, ContractError> {
        match &self.status {
            RegistrationStatus::Pending { config } => Ok(get_transfer_msg(to, config)?),
            RegistrationStatus::Approved => Ok(get_transfer_msg(to, &config.unwrap())?),
            _ => Err(ContractError::NoPendingRegistrationFound),
        }
    }
}

// Get the cosmos msg to transfer the fee based on the config.
fn get_transfer_msg(to: &Addr, config: &Config) -> StdResult<CosmosMsg> {
    Ok(match &config.fee_denom {
        CheckedDenom::Native(denom) => BankMsg::Send {
            to_address: to.to_string(),
            amount: vec![Coin {
                denom: denom.to_string(),
                amount: config.fee_amount,
            }],
        }
        .into(),
        CheckedDenom::Cw20(address) => WasmMsg::Execute {
            contract_addr: address.to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: to.to_string(),
                amount: config.fee_amount,
            })?,
            funds: vec![],
        }
        .into(),
    })
}
