use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg, WasmMsg};
use cw_hooks::Hooks;

/// An enum representing NFT staking hooks.
#[cw_serde]
pub enum NftStakeChangedHookMsg {
    Stake { addr: Addr, token_id: String },
    Unstake { addr: Addr, token_ids: Vec<String> },
}

/// Prepares NftStakeChangedHookMsg::Stake hook SubMsgs containing the address
/// and the token_id staked. The producer owns `reply_id`, must handle it, and
/// receives a reply for both successful and failed hook calls. See
/// [`crate::stake::stake_hook_reply_response`].
pub fn stake_nft_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    token_id: String,
    reply_id: u64,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&NftStakeChangedExecuteMsg::NftStakeChangeHook(
        NftStakeChangedHookMsg::Stake { addr, token_id },
    ))?;
    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::reply_always(execute, reply_id))
    })
}

/// Prepares NftStakeChangedHookMsg::Unstake hook SubMsgs containing the
/// address and the token_ids unstaked. The producer owns `reply_id`, must
/// handle it, and receives a reply for both successful and failed hook calls.
/// See [`crate::stake::stake_hook_reply_response`].
pub fn unstake_nft_hook_msgs(
    hooks: Hooks,
    storage: &dyn Storage,
    addr: Addr,
    token_ids: Vec<String>,
    reply_id: u64,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&NftStakeChangedExecuteMsg::NftStakeChangeHook(
        NftStakeChangedHookMsg::Unstake { addr, token_ids },
    ))?;

    hooks.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::reply_always(execute, reply_id))
    })
}

#[cw_serde]
pub enum NftStakeChangedExecuteMsg {
    NftStakeChangeHook(NftStakeChangedHookMsg),
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, testing::mock_dependencies, CosmosMsg, ReplyOn};

    use super::*;

    const REPLY_ID: u64 = 17;

    fn hooks_with_receiver(storage: &mut dyn Storage) -> Hooks<'static> {
        let hooks = Hooks::new("nft_stake_hooks");
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
        let token_id = "1".to_string();

        let messages = stake_nft_hook_msgs(
            hooks,
            &deps.storage,
            addr.clone(),
            token_id.clone(),
            REPLY_ID,
        )
        .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, REPLY_ID);
        assert_eq!(messages[0].reply_on, ReplyOn::Always);
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
    fn unstake_messages_reply_always_with_caller_id_and_unchanged_payload() {
        let mut deps = mock_dependencies();
        let hooks = hooks_with_receiver(&mut deps.storage);
        let addr = Addr::unchecked("staker");
        let token_ids = vec!["1".to_string(), "2".to_string()];

        let messages = unstake_nft_hook_msgs(
            hooks,
            &deps.storage,
            addr.clone(),
            token_ids.clone(),
            REPLY_ID,
        )
        .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, REPLY_ID);
        assert_eq!(messages[0].reply_on, ReplyOn::Always);
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
