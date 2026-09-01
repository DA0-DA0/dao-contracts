use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Response, StdResult, Storage, SubMsg, SubMsgResult, Uint128, WasmMsg,
};
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

/// Builds the producer's response to a stake or unstake hook reply.
///
/// A hook receiver must not be able to block staking or unstaking, and a hook
/// that fails must not be silently removed. Successful hook calls produce an
/// empty response. Failed hook calls leave the hook registered and are
/// surfaced as attributes on the producer's transaction so that the failure
/// is observable on-chain:
///
/// - `action`: `stake_hook_failed`
/// - `hook`: `stake` or `unstake`
/// - `error`: the error returned by the hook receiver
pub fn stake_hook_reply_response(hook: &str, result: SubMsgResult) -> Response {
    match result {
        SubMsgResult::Ok(_) => Response::new(),
        SubMsgResult::Err(error) => Response::new()
            .add_attribute("action", "stake_hook_failed")
            .add_attribute("hook", hook)
            .add_attribute("error", error),
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        attr, from_json, testing::mock_dependencies, CosmosMsg, ReplyOn, SubMsgResponse,
    };

    use super::*;

    const REPLY_ID: u64 = 17;

    fn hooks_with_receiver(storage: &mut dyn Storage) -> Hooks<'static> {
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
        let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: StakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            StakeChangedExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake { addr, amount })
        );
    }

    #[test]
    fn reply_response_reports_failures_without_erroring() {
        let response = stake_hook_reply_response("stake", SubMsgResult::Err("boom".to_string()));
        assert_eq!(
            response.attributes,
            vec![
                attr("action", "stake_hook_failed"),
                attr("hook", "stake"),
                attr("error", "boom"),
            ]
        );
        assert!(response.messages.is_empty());

        let response = stake_hook_reply_response(
            "unstake",
            SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        );
        assert!(response.attributes.is_empty());
        assert!(response.messages.is_empty());
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
        let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: StakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            StakeChangedExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Unstake { addr, amount })
        );
    }
}
