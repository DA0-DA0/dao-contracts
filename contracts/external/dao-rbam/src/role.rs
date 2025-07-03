use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Deps, DepsMut};
use cw_jsonfilter::{get_protobuf_messages, CwJsonFilter, FilterResult};
use prost::Message;
use prost_reflect::prost_types::FileDescriptorSet;

use crate::{
    helpers::{get_encoded_file_descriptor_set, get_next_id},
    state::{ASSIGNMENTS, AUTHORIZATIONS, AUTHORIZATION_FILE_DESCRIPTOR_SETS, ROLES},
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
    /// Optional `cw-jsonfilter` filter. If no filter is provided, no messages
    /// will be allowed, making it simply a symbolic authorization (which may be
    /// useful for external systems).
    pub filter: Option<serde_json::Value>,
    /// Protobuf messages that are referenced by the filter and need to be
    /// included when testing for message matches. This is internally managed
    /// and updated automatically when the filter is set.
    pub protobuf_messages: Vec<String>,
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
        let mut authorization = Authorization {
            id: get_next_id(deps.branch())?,
            role_id,
            name,
            metadata,
            filter,
            protobuf_messages: vec![],
            enabled,
        };
        authorization.sync_protobuf_messages(&mut deps)?;

        AUTHORIZATIONS.save(deps.storage, authorization.id, &authorization)?;

        Ok(authorization)
    }

    pub fn load(deps: &Deps, id: u64) -> Result<Authorization, ContractError> {
        AUTHORIZATIONS
            .load(deps.storage, id)
            .map_err(|_| ContractError::AuthorizationNotFound { id })
    }

    /// Whether or not the filter allows the given message.
    ///
    /// If `ignore_filter_error` is true, then the function will simply return
    /// false when a filter error occurs as if the filter failed. Otherwise,
    /// errors will be returned whenever a filter error occurs instead of
    /// returning false.
    ///
    /// If `protobuf_messages` is provided, then the function will prepare the
    /// specified protobuf messages for decoding. Otherwise, it will extract the
    /// protobuf messages referenced by the filter.
    pub fn filter_allows(
        filter: &serde_json::Value,
        file_descriptor_set: Option<FileDescriptorSet>,
        msg: &CosmosMsg,
        ignore_filter_error: bool,
    ) -> Result<bool, ContractError> {
        let msg_value = serde_json::to_value(msg)
            .map_err(|e| ContractError::JsonSerialization { err: e.to_string() })?;

        let fds = file_descriptor_set.map(|fd| vec![fd]).unwrap_or_default();
        let result = CwJsonFilter::new(fds).matches(filter, &msg_value);

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
                FilterResult::Fatal(error) => Err(ContractError::FilterError {
                    err: error.to_string(),
                }),
            }
        }
    }

    pub fn save(&self, deps: DepsMut) -> Result<(), ContractError> {
        AUTHORIZATIONS.save(deps.storage, self.id, self)?;
        Ok(())
    }

    /// Sync the protobuf messages referenced by the filter, ensuring that all
    /// protobuf messages are registered, and compute the needed file descriptor
    /// set. This is used when testing for message matches to provide the filter
    /// with the necessary protobuf descriptors for decoding.
    pub fn sync_protobuf_messages(&mut self, deps: &mut DepsMut) -> Result<(), ContractError> {
        if let Some(filter) = &self.filter {
            // Get the protobuf messages referenced by the filter.
            let protobuf_messages = get_protobuf_messages(filter)
                .into_iter()
                .collect::<Vec<_>>();

            // If the protobuf messages haven't changed, do nothing.
            if protobuf_messages == self.protobuf_messages {
                return Ok(());
            }

            // Update the protobuf messages.
            self.protobuf_messages = protobuf_messages;

            // If there are no protobuf messages, remove the file descriptor set
            // if it exists. Otherwise, compute and save it.
            match self.protobuf_messages.is_empty() {
                true => {
                    AUTHORIZATION_FILE_DESCRIPTOR_SETS.remove(deps.storage, self.id);
                }
                false => {
                    // Query for and save the file descriptor set.
                    let fds = get_encoded_file_descriptor_set(
                        &deps.as_ref(),
                        self.protobuf_messages.clone(),
                    )?;
                    AUTHORIZATION_FILE_DESCRIPTOR_SETS.save(deps.storage, self.id, &fds)?;
                }
            };
        }
        Ok(())
    }

    /// Whether or not the authorization allows the given message. If
    /// `ignore_filter_error` is true, then the function will simply return
    /// false when a filter error occurs as if the filter failed (otherwise an
    /// error is always returned on failure). This does NOT check if the
    /// authorization is enabled—that is the caller's responsibility.
    pub fn allows(
        &self,
        deps: &Deps,
        msg: &CosmosMsg,
        ignore_filter_error: bool,
    ) -> Result<bool, ContractError> {
        match (&self.filter, ignore_filter_error) {
            (Some(filter), _) => {
                let file_descriptor_set = if !self.protobuf_messages.is_empty() {
                    Some(FileDescriptorSet::decode(
                        AUTHORIZATION_FILE_DESCRIPTOR_SETS
                            .load(deps.storage, self.id)?
                            .as_slice(),
                    )?)
                } else {
                    None
                };

                Authorization::filter_allows(filter, file_descriptor_set, msg, ignore_filter_error)
            }
            (None, true) => Ok(false),
            (None, false) => Err(ContractError::NoAuthorizationFilterSet {}),
        }
    }
}
