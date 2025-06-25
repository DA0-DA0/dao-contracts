use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env};

use crate::{
    helpers::get_next_id,
    role::{Authorization, Role},
    state::LOG,
    ContractError,
};

#[cw_serde]
pub struct Action {
    /// Autoincrementing ID.
    pub id: u64,
    /// Address that executed the message.
    pub addr: Addr,
    /// Message that was executed.
    pub msg: CosmosMsg,
    /// Role ID used to perform this action.
    pub role_id: u64,
    /// Authorization ID used to perform this action.
    pub authorization_id: u64,
    /// The block height at which the action was performed.
    pub height: u64,
    /// The position of the transaction in the block.
    ///
    /// From cosmwasm-std::Env docs:
    ///
    /// The field is unset when the
    /// `MsgExecuteContract`/`MsgInstantiateContract`/`MsgMigrateContract` is
    /// not executed as part of a transaction.
    pub tx_index: Option<u32>,
}

impl Action {
    pub fn create(
        mut deps: DepsMut,
        env: &Env,
        addr: Addr,
        msg: CosmosMsg,
        role_id: u64,
        authorization_id: u64,
    ) -> Result<Action, ContractError> {
        let action = Action {
            id: get_next_id(deps.branch())?,
            addr: addr.clone(),
            msg,
            role_id,
            authorization_id,
            height: env.block.height,
            tx_index: env.transaction.as_ref().map(|tx| tx.index),
        };

        LOG.save(deps.storage, (addr, action.id), &action)?;

        Ok(action)
    }
}

#[cw_serde]
pub struct ActionToExecute {
    pub msg: CosmosMsg,
    pub role_id: u64,
    pub authorization_id: u64,
}

impl ActionToExecute {
    /// Validate the action can be executed and create an action from an
    /// action_to_execute. The caller must ensure the action message gets
    /// executed.
    pub fn initiate(
        self,
        mut deps: DepsMut,
        env: &Env,
        sender: &Addr,
    ) -> Result<Action, ContractError> {
        let ActionToExecute {
            msg,
            role_id,
            authorization_id,
        } = self;

        // Ensure the role and authorization exist.
        let role = Role::load(&deps.as_ref(), role_id)?;
        let authorization = Authorization::load(&deps.as_ref(), authorization_id)?;

        // Ensure authorization belongs to the role.
        if authorization.role_id != role.id {
            return Err(ContractError::AuthorizationRoleMismatch {});
        }

        // Ensure address has the role assigned.
        let assigned = Role::is_assigned(&deps.as_ref(), sender, role_id);
        if !assigned {
            return Err(ContractError::RoleNotAssigned {
                addr: sender.to_string(),
                role_id,
            });
        }

        // Ensure role is enabled.
        if !role.enabled {
            return Err(ContractError::RoleDisabled {});
        }

        // Ensure authorization is enabled.
        if !authorization.enabled {
            return Err(ContractError::AuthorizationDisabled {});
        }

        // Ensure message is allowed.
        let allowed = authorization.allows(&msg, false)?;
        // should never happen since ignore_filter_error is false
        if !allowed {
            return Err(ContractError::MsgNotAllowedByFilter {
                err: "unknown reason".to_string(),
            });
        }

        // Save and return the action.
        Action::create(
            deps.branch(),
            env,
            sender.clone(),
            msg,
            role_id,
            authorization_id,
        )
    }
}
