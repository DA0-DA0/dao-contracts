use cosmwasm_std::{to_binary, StdResult, Storage, SubMsg, WasmMsg};
use cw_controllers::Hooks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
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
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProposalHookExecuteMsg {
    ProposalHook(ProposalHookMsg),
}

pub fn new_proposal_hooks(hooks: Hooks, storage: &dyn Storage, id: u64) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&ProposalHookExecuteMsg::ProposalHook(
        ProposalHookMsg::NewProposal { id },
    ))?;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

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
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}
