use cosmwasm_std::{to_binary, StdResult, Storage, SubMsg, WasmMsg};
use indexable_hooks::Hooks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::reply::mask_proposal_hook_index;

/// The execute message variants that will be called on hook
/// receivers. For now, you must manually copy these variants over to
/// your `ExecuteMsg` enum. We'll fix that soon:
/// <https://github.com/DA0-DA0/dao-contracts/issues/459>
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProposalHookMsg {
    /// Hook that is fired when a new proposal is created.
    NewProposal { id: u64 },
    /// Hook that is fired when the status of a proposal changes. To
    /// support a variety of proposal modules, the status is provided
    /// as a string.
    ProposalStatusChanged {
        id: u64,
        old_status: String,
        new_status: String,
    },
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProposalHookExecuteMsg {
    ProposalHook(ProposalHookMsg),
}

/// Prepares new proposal hook messages. These messages reply on
/// error. A contract can use that to remove hooks which missfire.
pub fn new_proposal_hooks(hooks: Hooks, storage: &dyn Storage, id: u64) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&ProposalHookExecuteMsg::ProposalHook(
        ProposalHookMsg::NewProposal { id },
    ))?;
    let mut index: u64 = 0;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let masked_index = mask_proposal_hook_index(index);
        let tmp = SubMsg::reply_on_error(execute, masked_index);
        index += 1;
        Ok(tmp)
    })
}

/// Prepares proposal status hook messages. These messages reply on
/// error. A contract may use those replies to remove hooks which
/// missfire and prevent the proposal module from becoming locked.
pub fn proposal_status_changed_hooks(
    hooks: Hooks,
    storage: &dyn Storage,
    id: u64,
    old_status: String,
    new_status: String,
) -> StdResult<Vec<SubMsg>> {
    if old_status == new_status {
        return Ok(vec![]);
    }

    let msg = to_binary(&ProposalHookExecuteMsg::ProposalHook(
        ProposalHookMsg::ProposalStatusChanged {
            id,
            old_status,
            new_status,
        },
    ))?;
    let mut index: u64 = 0;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let masked_index = mask_proposal_hook_index(index);
        let tmp = SubMsg::reply_on_error(execute, masked_index);
        index += 1;
        Ok(tmp)
    })
}
