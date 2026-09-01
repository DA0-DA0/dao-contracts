use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg, Uint128, WasmMsg};
use cw_hooks::Hooks;

/// An enum representing staking hooks.
#[cw_serde]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, amount: Uint128 },
    Unstake { addr: Addr, amount: Uint128 },
}

/// Prepares StakeChangedHookMsg::Stake hook SubMsgs containing the address and
/// amount staked. The producer owns `reply_id`, must handle it, and receives a
/// reply for both successful and failed hook calls.
pub fn stake_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
    reply_id: u64,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Stake { addr, amount },
    ))?;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::reply_always(execute, reply_id))
    })
}

/// Prepares StakeChangedHookMsg::Unstake hook SubMsgs containing the address and
/// amount unstaked. The producer owns `reply_id`, must handle it, and receives a
/// reply for both successful and failed hook calls.
pub fn unstake_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
    reply_id: u64,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Unstake { addr, amount },
    ))?;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::reply_always(execute, reply_id))
    })
}

#[cw_serde]
pub enum StakeChangedExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, testing::mock_dependencies, ReplyOn};

    use super::*;

    const REPLY_ID: u64 = 17;

    fn hooks_with_receiver(storage: &mut dyn Storage) -> Hooks {
        let hooks = Hooks::new("stake_hooks");
        hooks
            .add_hook(storage, Addr::unchecked("receiver"))
            .unwrap();
        hooks
    }

    #[test]
    fn stake_messages_reply_always_with_caller_id_and_unchanged_payload() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receiver(&mut deps.storage);
        let addr = Addr::unchecked("staker");
        let amount = Uint128::new(42);

        let messages =
            stake_hook_msgs(hooks, &deps.storage, addr.clone(), amount, REPLY_ID).unwrap();

        assert_eq!(messages[0].id, REPLY_ID);
        assert_eq!(messages[0].reply_on, ReplyOn::Always);
        let WasmMsg::Execute { msg, .. } = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: StakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            StakeChangedExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake { addr, amount })
        );
    }

    #[test]
    fn unstake_messages_reply_always_with_caller_id_and_unchanged_payload() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receiver(&mut deps.storage);
        let addr = Addr::unchecked("staker");
        let amount = Uint128::new(24);

        let messages =
            unstake_hook_msgs(hooks, &deps.storage, addr.clone(), amount, REPLY_ID).unwrap();

        assert_eq!(messages[0].id, REPLY_ID);
        assert_eq!(messages[0].reply_on, ReplyOn::Always);
        let WasmMsg::Execute { msg, .. } = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: StakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            StakeChangedExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Unstake { addr, amount })
        );
    }
}
