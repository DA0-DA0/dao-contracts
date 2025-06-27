use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Deps, DepsMut};
use cw_jsonfilter::{CwJsonFilter, FilterResult};

use crate::{
    helpers::get_next_id,
    state::{ASSIGNMENTS, AUTHORIZATIONS, ROLES},
    ContractError,
};

#[cw_serde]
pub struct Role {
    /// Autoincrementing ID.
    pub id: u64,
    /// Name for the role (ideally unique, but not enforced).
    pub name: String,
    /// Optional metadata for the role. This should either be a JSON object or
    /// IPFS hash.
    pub metadata: Option<String>,
    /// Whether or not the role and all its authorizations are enabled.
    pub enabled: bool,
}

#[cw_serde]
pub struct Authorization {
    /// Autoincrementing ID.
    pub id: u64,
    /// Role ID.
    pub role_id: u64,
    /// Name for the authorization (ideally unique, but not enforced).
    pub name: String,
    /// Optional metadata for the authorization. This should either be a JSON
    /// object or IPFS hash.
    pub metadata: Option<String>,
    /// Optional `jsonfilter` filter. If no filter is provided, no messages will
    /// be allowed, making it simply a symbolic authorization (which may be
    /// useful for external systems).
    pub filter: Option<serde_json::Value>,
    /// Whether or not the authorization is enabled.
    pub enabled: bool,
}

impl Role {
    pub fn create(
        mut deps: DepsMut,
        name: String,
        metadata: Option<String>,
        enabled: bool,
    ) -> Result<Role, ContractError> {
        let role = Role {
            id: get_next_id(deps.branch())?,
            name,
            metadata,
            enabled,
        };

        ROLES.save(deps.storage, role.id, &role)?;

        Ok(role)
    }

    pub fn exists(deps: &Deps, id: u64) -> bool {
        ROLES.has(deps.storage, id)
    }

    pub fn ensure_exists(deps: &Deps, id: u64) -> Result<(), ContractError> {
        if !Role::exists(deps, id) {
            return Err(ContractError::RoleNotFound { id });
        }
        Ok(())
    }

    pub fn load(deps: &Deps, id: u64) -> Result<Role, ContractError> {
        ROLES
            .load(deps.storage, id)
            .map_err(|_| ContractError::RoleNotFound { id })
    }

    pub fn is_assigned(deps: &Deps, addr: &Addr, role_id: u64) -> bool {
        ASSIGNMENTS.has(deps.storage, (addr.clone(), role_id))
    }

    pub fn assign(deps: DepsMut, addr: &Addr, role_id: u64) -> Result<(), ContractError> {
        let assignment = (addr.clone(), role_id);
        ASSIGNMENTS.save(deps.storage, assignment, addr)?;
        Ok(())
    }

    pub fn revoke(deps: DepsMut, addr: &Addr, role_id: u64) -> Result<(), ContractError> {
        ASSIGNMENTS.remove(deps.storage, (addr.clone(), role_id))?;
        Ok(())
    }

    pub fn save(&self, deps: DepsMut) -> Result<(), ContractError> {
        ROLES.save(deps.storage, self.id, self)?;
        Ok(())
    }
}

impl Authorization {
    pub fn create(
        mut deps: DepsMut,
        role_id: u64,
        name: String,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: bool,
    ) -> Result<Authorization, ContractError> {
        let authorization = Authorization {
            id: get_next_id(deps.branch())?,
            role_id,
            name,
            metadata,
            filter,
            enabled,
        };

        AUTHORIZATIONS.save(deps.storage, authorization.id, &authorization)?;

        Ok(authorization)
    }

    pub fn load(deps: &Deps, id: u64) -> Result<Authorization, ContractError> {
        AUTHORIZATIONS
            .load(deps.storage, id)
            .map_err(|_| ContractError::AuthorizationNotFound { id })
    }

    pub fn save(&self, deps: DepsMut) -> Result<(), ContractError> {
        AUTHORIZATIONS.save(deps.storage, self.id, self)?;
        Ok(())
    }

    /// Whether or not the filter allows the given message. If
    /// `ignore_filter_error` is true, then the function will simply return
    /// false when a filter error occurs as if the filter failed. Otherwise,
    /// errors will be returned whenever a filter error occurs instead of
    /// returning false.
    pub fn filter_allows(
        filter: &serde_json::Value,
        msg: &CosmosMsg,
        ignore_filter_error: bool,
    ) -> Result<bool, ContractError> {
        let msg_value = serde_json::to_value(msg)
            .map_err(|e| ContractError::JsonSerialization { err: e.to_string() })?;
        let result = CwJsonFilter::check(filter, &msg_value);

        if ignore_filter_error {
            // Treat filter errors as failures.
            Ok(result.is_pass())
        } else {
            // Pass through filter errors.
            match result {
                FilterResult::Pass => Ok(true),
                FilterResult::Fail(error) => Err(ContractError::MsgNotAllowedByFilter {
                    err: error.to_string(),
                }),
                FilterResult::Fatal(error) => Err(ContractError::FilterInvalid {
                    err: error.to_string(),
                }),
            }
        }
    }

    /// Whether or not the authorization allows the given message. If
    /// `ignore_filter_error` is true, then the function will simply return
    /// false when a filter error occurs as if the filter failed (otherwise an
    /// error is always returned on failure). This does NOT check if the
    /// authorization is enabled—that is the caller's responsibility.
    pub fn allows(
        &self,
        msg: &CosmosMsg,
        ignore_filter_error: bool,
    ) -> Result<bool, ContractError> {
        match (&self.filter, ignore_filter_error) {
            (Some(filter), _) => Authorization::filter_allows(filter, msg, ignore_filter_error),
            (None, true) => Ok(false),
            (None, false) => Err(ContractError::NoAuthorizationFilterSet {}),
        }
    }
}
