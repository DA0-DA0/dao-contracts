use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Deps, DepsMut, SubMsg, WasmMsg};
use cw_protobuf_registry::protobuf::get_protobuf_messages;

use crate::{
    contract::PREPARE_PROTOBUF_REGISTRY_REPLY_ID,
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
    /// Optional `cw-jsonfilter` filter. If no filter is provided, no messages
    /// will be allowed, making it simply a symbolic authorization (which may be
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
    // The cosmos messages returned should be executed to prepare the protobuf
    // messages for decoding.
    pub fn create(
        mut deps: DepsMut,
        protobuf_registry: &Option<Addr>,
        role_id: u64,
        name: String,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: bool,
    ) -> Result<(Authorization, Vec<SubMsg>), ContractError> {
        let authorization = Authorization {
            id: get_next_id(deps.branch())?,
            role_id,
            name,
            metadata,
            filter,
            enabled,
        };
        let messages = authorization.get_protobuf_message_preparation_submsgs(protobuf_registry)?;

        AUTHORIZATIONS.save(deps.storage, authorization.id, &authorization)?;

        Ok((authorization, messages))
    }

    pub fn load(deps: &Deps, id: u64) -> Result<Authorization, ContractError> {
        AUTHORIZATIONS
            .load(deps.storage, id)
            .map_err(|_| ContractError::AuthorizationNotFound { id })
    }

    /// Whether or not the filter allows the given message.
    ///
    /// If `ignore_filter_error` is true, the function will simply return false
    /// when the filter errors as if it failed. Otherwise, specific errors will
    /// be returned whenever a filter fails or errors, and the only Ok value
    /// will be true if the filter passes.
    pub fn filter_allows(
        deps: &Deps,
        filter_contract: &Addr,
        filter: serde_json::Value,
        msg: CosmosMsg,
        ignore_filter_error: bool,
    ) -> Result<bool, ContractError> {
        let result = deps
            .querier
            .query_wasm_smart::<cw_filter::msg::FilterResponse>(
                filter_contract,
                &cw_filter::msg::QueryMsg::Filter { filter, msg },
            )
            .map_err(|e| ContractError::FilterContractQueryError {
                error: e.to_string(),
            })?;

        // Treat filter errors as failures, just returning a boolean.
        if ignore_filter_error {
            Ok(result == cw_filter::msg::FilterResponse::Pass {})
        } else {
            // Pass through errors, only returning true or an error.
            match result {
                cw_filter::msg::FilterResponse::Pass {} => Ok(true),
                cw_filter::msg::FilterResponse::Fail { reason } => {
                    Err(ContractError::MsgNotAllowedByFilter { err: reason })
                }
                cw_filter::msg::FilterResponse::Fatal { reason } => {
                    Err(ContractError::FilterError { err: reason })
                }
            }
        }
    }

    pub fn save(&self, deps: DepsMut) -> Result<(), ContractError> {
        AUTHORIZATIONS.save(deps.storage, self.id, self)?;
        Ok(())
    }

    /// Get the submessages to prepare protobufs for future decoding, if
    /// necessary. Preparing makes the protobuf registry more efficient when
    /// decoding protobufs during matching.
    pub fn get_protobuf_message_preparation_submsgs(
        &self,
        protobuf_registry: &Option<Addr>,
    ) -> Result<Vec<SubMsg>, ContractError> {
        if let Some(filter) = &self.filter {
            // Get the protobuf messages referenced by the filter.
            let protobuf_messages = get_protobuf_messages(filter)
                .into_iter()
                .collect::<Vec<_>>();

            // If there are protobuf messages and the protobuf registry is
            // registered, prepare them.
            if !protobuf_messages.is_empty() {
                if let Some(protobuf_registry) = protobuf_registry {
                    return Ok(vec![SubMsg::reply_on_error(
                        WasmMsg::Execute {
                            contract_addr: protobuf_registry.to_string(),
                            msg: to_json_binary(&cw_protobuf_registry::msg::ExecuteMsg::Prepare {
                                messages: protobuf_messages,
                            })?,
                            funds: vec![],
                        },
                        PREPARE_PROTOBUF_REGISTRY_REPLY_ID,
                    )]);
                }
            };
        }

        Ok(vec![])
    }

    /// Whether or not the authorization allows the given message. If
    /// `ignore_filter_error` is true, then the function will simply return
    /// false when a filter error occurs as if the filter failed (otherwise an
    /// error is always returned on failure). This does NOT check if the
    /// authorization is enabled—that is the caller's responsibility.
    pub fn allows(
        &self,
        deps: &Deps,
        filter_contract: &Addr,
        msg: CosmosMsg,
        ignore_filter_error: bool,
    ) -> Result<bool, ContractError> {
        match (&self.filter, ignore_filter_error) {
            (Some(filter), _) => {
                Authorization::filter_allows(deps, filter_contract, filter.clone(), msg, ignore_filter_error)
            }
            (None, true) => Ok(false),
            (None, false) => Err(ContractError::NoAuthorizationFilterSet {}),
        }
    }
}
