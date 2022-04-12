use crate::state::HOOKS;
use cosmwasm_std::{to_binary, Addr, StdResult, Storage, SubMsg, Uint128, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StakeChangedHookMsg {
    Stake {
        addr: Addr,
        amount: Uint128,
        staked_addresses_count: u64,
    },
    Unstake {
        addr: Addr,
        amount: Uint128,
        staked_addresses_count: u64,
    },
}

pub fn stake_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
    staked_addresses_count: usize,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Stake {
            addr,
            amount,
            staked_addresses_count: staked_addresses_count as u64,
        },
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
    amount: Uint128,
    staked_addresses_count: usize,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Unstake {
            addr,
            amount,
            staked_addresses_count: staked_addresses_count as u64,
        },
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
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum StakeChangedExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
}
