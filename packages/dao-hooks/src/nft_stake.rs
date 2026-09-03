use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg};
use cw_hooks::Hooks;

use crate::stake::{prepare_hook_msgs, STAKE_HOOK_REPLY_ID_BASE, UNSTAKE_HOOK_REPLY_ID_BASE};

/// An enum representing NFT staking hooks.
#[cw_serde]
pub enum NftStakeChangedHookMsg {
    Stake { addr: Addr, token_id: String },
    Unstake { addr: Addr, token_ids: Vec<String> },
}

/// Prepares NftStakeChangedHookMsg::Stake hook SubMsgs containing the address
/// and the token_id staked.
///
/// Each hook is dispatched with `reply_always` so that a failing receiver
/// cannot abort the producer's transaction. Producers must handle the reply
/// with [`crate::stake::handle_stake_hook_reply`].
pub fn stake_nft_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    token_id: String,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&NftStakeChangedExecuteMsg::NftStakeChangeHook(
        NftStakeChangedHookMsg::Stake { addr, token_id },
    ))?;
    prepare_hook_msgs(hooks, storage, msg, STAKE_HOOK_REPLY_ID_BASE)
}

/// Prepares NftStakeChangedHookMsg::Unstake hook SubMsgs containing the
/// address and the token_ids unstaked.
///
/// Each hook is dispatched with `reply_always` so that a failing receiver
/// cannot abort the producer's transaction. Producers must handle the reply
/// with [`crate::stake::handle_stake_hook_reply`].
pub fn unstake_nft_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    token_ids: Vec<String>,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&NftStakeChangedExecuteMsg::NftStakeChangeHook(
        NftStakeChangedHookMsg::Unstake { addr, token_ids },
    ))?;
    prepare_hook_msgs(hooks, storage, msg, UNSTAKE_HOOK_REPLY_ID_BASE)
}

#[cw_serde]
pub enum NftStakeChangedExecuteMsg {
    NftStakeChangeHook(NftStakeChangedHookMsg),
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, testing::mock_dependencies, CosmosMsg, ReplyOn, WasmMsg};

    use super::*;

    fn hooks_with_receivers(storage: &mut dyn Storage, addrs: &[&str]) -> Hooks<'static> {
        let hooks = Hooks::new("nft_stake_hooks");
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
        let token_id = "1".to_string();

        let messages =
            stake_nft_hook_msgs(hooks, &deps.storage, addr.clone(), token_id.clone()).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, STAKE_HOOK_REPLY_ID_BASE);
        assert_eq!(messages[1].id, STAKE_HOOK_REPLY_ID_BASE + 1);
        for message in &messages {
            assert_eq!(message.reply_on, ReplyOn::Always);
        }
        let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: NftStakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            NftStakeChangedExecuteMsg::NftStakeChangeHook(NftStakeChangedHookMsg::Stake {
                addr,
                token_id
            })
        );
    }

    #[test]
    fn unstake_messages_reply_always_with_indexed_ids_and_unchanged_payload() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receivers(&mut deps.storage, &["first", "second"]);
        let addr = Addr::unchecked("staker");
        let token_ids = vec!["1".to_string(), "2".to_string()];

        let messages =
            unstake_nft_hook_msgs(hooks, &deps.storage, addr.clone(), token_ids.clone()).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, UNSTAKE_HOOK_REPLY_ID_BASE);
        assert_eq!(messages[1].id, UNSTAKE_HOOK_REPLY_ID_BASE + 1);
        for message in &messages {
            assert_eq!(message.reply_on, ReplyOn::Always);
        }
        let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &messages[0].msg else {
            panic!("expected Wasm execute message")
        };
        let payload: NftStakeChangedExecuteMsg = from_json(msg).unwrap();
        assert_eq!(
            payload,
            NftStakeChangedExecuteMsg::NftStakeChangeHook(NftStakeChangedHookMsg::Unstake {
                addr,
                token_ids
            })
        );
    }
}
