use crate::state::HOOKS;
use cosmwasm_std::{to_binary, Addr, StdResult, Storage, SubMsg, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, token_id: String },
    Unstake { addr: Addr, token_id: String },
}

pub fn stake_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    token_id: String,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Stake { addr, token_id },
    ))?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

pub fn unstake_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    token_id: String,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Unstake { addr, token_id },
    ))?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum StakeChangedExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
}
