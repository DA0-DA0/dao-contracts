use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg, Uint128, WasmMsg};
use cw_hooks::Hooks;

use crate::state::Vote;

/// Hook fired from the orchestrator on `PlaceVotes`. Subscribers can use it
/// to drive participation rewards, off-chain notifications, analytics, etc.
///
/// Payload is the *new* state after the vote: `votes` may be empty (the
/// voter abstained / cleared their position). `voting_power` is the power
/// the orchestrator read for the voter on this call.
#[cw_serde]
pub enum GaugeVoteHookMsg {
    NewVotes {
        gauge_id: u64,
        voter: String,
        votes: Vec<Vote>,
        voting_power: Uint128,
        height: u64,
    },
}

/// Outer envelope that subscribed contracts will receive. Match on
/// `GaugeVoteHook(..)` to handle.
#[cw_serde]
pub enum GaugeVoteHookExecuteMsg {
    GaugeVoteHook(GaugeVoteHookMsg),
}

/// Build the `SubMsg` list for every currently-registered hook. Each submsg
/// uses `reply_on_error` with the hook's index as its reply ID, so the
/// `reply` entry-point can auto-unregister failing subscribers without
/// blocking the underlying `PlaceVotes` call.
pub fn new_vote_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    gauge_id: u64,
    voter: Addr,
    votes: Vec<Vote>,
    voting_power: Uint128,
    height: u64,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&GaugeVoteHookExecuteMsg::GaugeVoteHook(
        GaugeVoteHookMsg::NewVotes {
            gauge_id,
            voter: voter.into_string(),
            votes,
            voting_power,
            height,
        },
    ))?;
    let mut idx: u64 = 0;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let sub = SubMsg::reply_on_error(execute, idx);
        idx += 1;
        Ok(sub)
    })
}
