#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, StdResult, Storage, SubMsg, WasmMsg};
use cwd_hooks::Hooks;
use cwd_voting::reply::mask_proposal_hook_index;

#[cw_serde]
pub enum ProposalHookMsg {
    NewProposal {
        id: u64,
        proposer: String,
    },
    ProposalStatusChanged {
        id: u64,
        old_status: String,
        new_status: String,
    },
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum ProposalHookExecuteMsg {
    ProposalHook(ProposalHookMsg),
}

/// Prepares new proposal hook messages. These messages reply on error
/// and have even reply IDs.
/// IDs are set to even numbers to then be interleaved with the vote hooks.
pub fn new_proposal_hooks(
    hooks: Hooks,
    storage: &dyn Storage,
    id: u64,
    proposer: &str,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&ProposalHookExecuteMsg::ProposalHook(
        ProposalHookMsg::NewProposal {
            id,
            proposer: proposer.to_string(),
        },
    ))?;

    let mut index: u64 = 0;
    let messages = hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let masked_index = mask_proposal_hook_index(index);
        let tmp = SubMsg::reply_on_error(execute, masked_index);
        index += 1;
        Ok(tmp)
    })?;

    Ok(messages)
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
    let messages = hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let masked_index = mask_proposal_hook_index(index);
        let tmp = SubMsg::reply_on_error(execute, masked_index);
        index += 1;
        Ok(tmp)
    })?;

    Ok(messages)
}
