use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, StdResult, Storage, SubMsg, WasmMsg};
use cw_hooks::Hooks;

#[cw_serde]
pub enum NftStakeChangedHookMsg {
    Stake { addr: Addr, token_id: String },
    Unstake { addr: Addr, token_ids: Vec<String> },
}

pub fn stake_nft_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    token_id: String,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&NftStakeChangedExecuteMsg::StakeChangeHook(
        NftStakeChangedHookMsg::Stake { addr, token_id },
    ))?;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

pub fn unstake_nft_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    token_ids: Vec<String>,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&NftStakeChangedExecuteMsg::StakeChangeHook(
        NftStakeChangedHookMsg::Unstake { addr, token_ids },
    ))?;

    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

#[cw_serde]
pub enum NftStakeChangedExecuteMsg {
    NftStakeChangeHook(NftStakeChangedHookMsg),
}
