use cosmwasm_std::{to_binary, StdResult, Storage, SubMsg, WasmMsg};
use indexable_hooks::Hooks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProposalHookMsg {
    NewProposal {
        id: u64,
    },
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
/// Prepares new proposal hook messages. These messages reply on error
/// and have even reply IDs.
/// IDs are set to even numbers to then be interleaved with the vote hooks.
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
        let tmp = SubMsg::reply_on_error(execute, index * 2);
        index += 1;
        Ok(tmp)
    })
}

/// Prepares proposal status hook messages. These messages reply on error
/// and have even reply IDs.
/// IDs are set to even numbers to then be interleaved with the vote hooks.
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
        let tmp = SubMsg::reply_on_error(execute, index * 2);
        index += 1;
        Ok(tmp)
    })
}
