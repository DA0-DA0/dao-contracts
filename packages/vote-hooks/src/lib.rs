use cosmwasm_std::{to_binary, StdResult, Storage, SubMsg, WasmMsg};
use indexable_hooks::Hooks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VoteHookMsg {
    NewVote {
        proposal_id: u64,
        voter: String,
        vote: String,
    },
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
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
        let tmp = SubMsg::reply_on_error(execute, index * 2 + 1);
        index += 1;
        Ok(tmp)
    })
}
