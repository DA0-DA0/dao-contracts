use cosmwasm_std::{Addr, CosmosMsg, DepsMut, StdResult, Storage, SubMsg, to_binary, Uint128, WasmMsg};
use crate::state::HOOKS;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StakeChangedExecuteMsg {
    Stake {addr: Addr, amount: Uint128},
    Unstake {addr: Addr, amount: Uint128}
}

pub fn stake_hook_msgs(storage: &dyn Storage, addr: Addr, amount: Uint128) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::Stake { addr, amount })?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![]
        };
        Ok(SubMsg::new(execute))
    })
}

pub fn unstake_hook_msgs(storage: &dyn Storage, addr: Addr, amount: Uint128) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::Unstake { addr, amount })?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![]
        };
        Ok(SubMsg::new(execute))
    })
}