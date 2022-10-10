use cosmwasm_std::{to_binary, StdResult, Storage, SubMsg, WasmMsg};
use cwd_hooks::Hooks;
use cwd_voting::reply::mask_vote_hook_index;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VoteHookMsg {
    NewVote {
        proposal_id: u64,
        voter: String,
        vote: String,
    },
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VoteHookExecuteMsg {
    VoteHook(VoteHookMsg),
}

/// Prepares new vote hook messages. These messages reply on error
/// and have even reply IDs.
/// IDs are set to odd numbers to then be interleaved with the proposal hooks.
pub fn new_vote_hooks(
    hooks: Hooks,
    storage: &dyn Storage,
    proposal_id: u64,
    voter: String,
    vote: String,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&VoteHookExecuteMsg::VoteHook(VoteHookMsg::NewVote {
        proposal_id,
        voter,
        vote,
    }))?;
    let mut index: u64 = 0;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let masked_index = mask_vote_hook_index(index);
        let tmp = SubMsg::reply_on_error(execute, masked_index);
        index += 1;
        Ok(tmp)
    })
}
