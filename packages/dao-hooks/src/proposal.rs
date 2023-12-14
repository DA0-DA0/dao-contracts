use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Empty, StdResult, Storage, SubMsg, WasmMsg};
use cw_hooks::Hooks;
use dao_voting::{
    pre_propose::ProposalCreationPolicy,
    reply::{failed_pre_propose_module_hook_id, mask_proposal_hook_index},
    status::Status,
};

/// An enum representing proposal hook messages.
/// Either a new propsoal hook, fired when a new proposal is created,
/// or a proposal status hook, fired when a proposal changes status.
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

/// Prepares new proposal hook messages. These messages reply on error
/// and have even reply IDs.
/// IDs are set to even numbers to then be interleaved with the vote hooks.
pub fn new_proposal_hooks(
    hooks: Hooks,
    storage: &dyn Storage,
    id: u64,
    proposer: &str,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&ProposalHookExecuteMsg::ProposalHook(
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

    let msg = to_json_binary(&ProposalHookExecuteMsg::ProposalHook(
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

/// Message type used for firing hooks to a proposal module's pre-propose
/// module, if one is installed.
pub type PreProposeHookMsg = dao_pre_propose_base::msg::ExecuteMsg<Empty, Empty>;

/// Adds prepropose / deposit module hook which will handle deposit refunds.
pub fn proposal_completed_hooks(
    proposal_creation_policy: ProposalCreationPolicy,
    proposal_id: u64,
    new_status: Status,
) -> StdResult<Vec<SubMsg>> {
    let mut hooks: Vec<SubMsg> = vec![];
    match proposal_creation_policy {
        ProposalCreationPolicy::Anyone {} => (),
        ProposalCreationPolicy::Module { addr } => {
            let msg = to_json_binary(&PreProposeHookMsg::ProposalCompletedHook {
                proposal_id,
                new_status,
            })?;
            hooks.push(SubMsg::reply_on_error(
                WasmMsg::Execute {
                    contract_addr: addr.into_string(),
                    msg,
                    funds: vec![],
                },
                failed_pre_propose_module_hook_id(),
            ));
        }
    };
    Ok(hooks)
}

#[cw_serde]
pub enum ProposalHookExecuteMsg {
    ProposalHook(ProposalHookMsg),
}
