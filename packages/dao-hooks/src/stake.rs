use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Deps, Reply, Response, StdResult, Storage, SubMsg, SubMsgResult, Uint128,
    WasmMsg,
};
use cw_hooks::Hooks;

/// First reply ID used by stake hooks. The hook at index `i` in the producer's
/// registry is dispatched with `STAKE_HOOK_REPLY_ID_BASE + i`.
pub const STAKE_HOOK_REPLY_ID_BASE: u64 = 1 << 32;

/// First reply ID used by unstake hooks. The hook at index `i` in the
/// producer's registry is dispatched with `UNSTAKE_HOOK_REPLY_ID_BASE + i`.
pub const UNSTAKE_HOOK_REPLY_ID_BASE: u64 = 1 << 33;

/// Number of reply IDs reserved for each of the two ranges above. Producers
/// must not use reply IDs inside either range.
pub const HOOK_REPLY_ID_RANGE: u64 = 1 << 32;

/// An enum representing staking hooks.
#[cw_serde]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, amount: Uint128 },
    Unstake { addr: Addr, amount: Uint128 },
}

/// Prepares StakeChangedHookMsg::Stake hook SubMsgs containing the address and
/// amount staked.
///
/// Each hook is dispatched with `reply_always` so that a failing receiver
/// cannot abort the producer's transaction. Producers must handle the reply
/// with [`handle_stake_hook_reply`].
pub fn stake_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Stake { addr, amount },
    ))?;
    prepare_hook_msgs(hooks, storage, msg, STAKE_HOOK_REPLY_ID_BASE)
}

/// Prepares StakeChangedHookMsg::Unstake hook SubMsgs containing the address
/// and amount unstaked.
///
/// Each hook is dispatched with `reply_always` so that a failing receiver
/// cannot abort the producer's transaction. Producers must handle the reply
/// with [`handle_stake_hook_reply`].
pub fn unstake_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Unstake { addr, amount },
    ))?;
    prepare_hook_msgs(hooks, storage, msg, UNSTAKE_HOOK_REPLY_ID_BASE)
}

/// Dispatches `msg` to every registered hook with `reply_always`, tagging each
/// submessage with `base + <index of the hook in the registry>` so the reply
/// handler can name the receiver that failed.
pub(crate) fn prepare_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    msg: cosmwasm_std::Binary,
    base: u64,
) -> StdResult<Vec<SubMsg>> {
    let mut index = 0u64;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        let sub_msg = SubMsg::reply_always(execute, base + index);
        index += 1;
        Ok(sub_msg)
    })
}

#[cw_serde]
pub enum StakeChangedExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
}

/// Handles a reply from a stake, unstake, NFT stake or NFT unstake hook.
///
/// Returns `Ok(None)` when `msg.id` is not one of the hook reply IDs, so a
/// producer with reply IDs of its own can fall through to them.
///
/// A hook receiver must not be able to block staking or unstaking, and a hook
/// that fails must not be silently removed. A successful hook produces an
/// empty response. A failed hook leaves the receiver registered, lets the
/// staking transaction succeed, and records the failure as attributes:
///
/// - `action`: `stake_hook_failed`
/// - `hook`: `stake` or `unstake`
/// - `addr`: the receiver that failed
/// - `error`: the error reported for the receiver
///
/// Note that CosmWasm chains redact submessage errors before they reach a
/// reply handler, so on chain `error` is a codespace and code rather than the
/// receiver's own message. `addr` is therefore the identifier a DAO needs in
/// order to act on the failure, and is resolved from the producer's registry
/// by index. It is best effort: if the registry has since shrunk, `unknown` is
/// reported instead of an address.
pub fn handle_stake_hook_reply(
    hooks: Hooks,
    deps: Deps,
    msg: &Reply,
) -> StdResult<Option<Response>> {
    let in_range = |base: u64| {
        msg.id
            .checked_sub(base)
            .filter(|index| *index < HOOK_REPLY_ID_RANGE)
    };

    let (hook, index) = if let Some(index) = in_range(STAKE_HOOK_REPLY_ID_BASE) {
        ("stake", index)
    } else if let Some(index) = in_range(UNSTAKE_HOOK_REPLY_ID_BASE) {
        ("unstake", index)
    } else {
        return Ok(None);
    };

    let error = match &msg.result {
        SubMsgResult::Ok(_) => return Ok(Some(Response::new())),
        SubMsgResult::Err(error) => error,
    };

    let addr = hooks
        .query_hooks(deps)?
        .hooks
        .get(index as usize)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Some(
        Response::new()
            .add_attribute("action", "stake_hook_failed")
            .add_attribute("hook", hook)
            .add_attribute("addr", addr)
            .add_attribute("error", error),
    ))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        attr, from_json, testing::mock_dependencies, CosmosMsg, ReplyOn, SubMsgResponse,
    };

    use super::*;

    fn hooks_with_receivers(storage: &mut dyn Storage, addrs: &[&str]) -> Hooks<'static> {
        let hooks = Hooks::new("stake_hooks");
        for addr in addrs {
            hooks.add_hook(storage, Addr::unchecked(*addr)).unwrap();
        }
        hooks
    }

    #[test]
    fn stake_messages_reply_always_with_indexed_ids_and_unchanged_payload() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receivers(&mut deps.storage, &["first", "second"]);
        let addr = Addr::unchecked("staker");
        let amount = Uint128::new(42);

        let messages = stake_hook_msgs(hooks, &deps.storage, addr.clone(), amount).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, STAKE_HOOK_REPLY_ID_BASE);
        assert_eq!(messages[1].id, STAKE_HOOK_REPLY_ID_BASE + 1);
        for message in &messages {
            assert_eq!(message.reply_on, ReplyOn::Always);
        }
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
    fn unstake_messages_reply_always_with_indexed_ids_and_unchanged_payload() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receivers(&mut deps.storage, &["first", "second"]);
        let addr = Addr::unchecked("staker");
        let amount = Uint128::new(24);

        let messages = unstake_hook_msgs(hooks, &deps.storage, addr.clone(), amount).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, UNSTAKE_HOOK_REPLY_ID_BASE);
        assert_eq!(messages[1].id, UNSTAKE_HOOK_REPLY_ID_BASE + 1);
        for message in &messages {
            assert_eq!(message.reply_on, ReplyOn::Always);
        }
        let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: StakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            StakeChangedExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Unstake { addr, amount })
        );
    }

    #[test]
    fn reply_names_the_receiver_that_failed() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receivers(&mut deps.storage, &["first", "second"]);

        let response = handle_stake_hook_reply(
            hooks,
            deps.as_ref(),
            &Reply {
                // the chain redacts the receiver's error to a codespace and
                // code before it reaches us, so the address carries the signal.
                id: UNSTAKE_HOOK_REPLY_ID_BASE + 1,
                result: SubMsgResult::Err("codespace: wasm, code: 5".to_string()),
            },
        )
        .unwrap()
        .expect("id is in the unstake hook range");

        assert_eq!(
            response.attributes,
            vec![
                attr("action", "stake_hook_failed"),
                attr("hook", "unstake"),
                attr("addr", "second"),
                attr("error", "codespace: wasm, code: 5"),
            ]
        );
        assert!(response.messages.is_empty());
    }

    #[test]
    fn reply_reports_unknown_when_the_receiver_is_no_longer_registered() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receivers(&mut deps.storage, &["only"]);

        let response = handle_stake_hook_reply(
            hooks,
            deps.as_ref(),
            &Reply {
                id: STAKE_HOOK_REPLY_ID_BASE + 7,
                result: SubMsgResult::Err("codespace: wasm, code: 5".to_string()),
            },
        )
        .unwrap()
        .expect("id is in the stake hook range");

        assert_eq!(response.attributes[2], attr("addr", "unknown"));
    }

    #[test]
    fn reply_is_empty_on_success_and_ignores_foreign_ids() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receivers(&mut deps.storage, &["only"]);

        let response = handle_stake_hook_reply(
            hooks,
            deps.as_ref(),
            &Reply {
                id: STAKE_HOOK_REPLY_ID_BASE,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap()
        .expect("id is in the stake hook range");
        assert!(response.attributes.is_empty());
        assert!(response.messages.is_empty());

        // A producer's own reply IDs must fall through untouched.
        for id in [0, 1, 2, 3, HOOK_REPLY_ID_RANGE - 1] {
            assert!(handle_stake_hook_reply(
                Hooks::new("stake_hooks"),
                deps.as_ref(),
                &Reply {
                    id,
                    result: SubMsgResult::Err("irrelevant".to_string()),
                },
            )
            .unwrap()
            .is_none());
        }
    }
}
