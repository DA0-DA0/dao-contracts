use crate::state::HOOKS;
use cosmwasm_std::{to_binary, Addr, StdResult, Storage, SubMsg, WasmMsg};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use schemars::JsonSchema;

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq, JsonSchema, Debug))]
#[serde(rename_all = "snake_case")]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, token_id: String },
    Unstake { addr: Addr, token_ids: Vec<String> },
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
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

pub fn unstake_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    token_ids: Vec<String>,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_binary(&StakeChangedExecuteMsg::StakeChangeHook(
        StakeChangedHookMsg::Unstake { addr, token_ids },
    ))?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.into_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq, JsonSchema, Debug))]
#[serde(rename_all = "snake_case")]
enum StakeChangedExecuteMsg {
    StakeChangeHook(StakeChangedHookMsg),
}

#[cfg(test)]
mod tests {
    use crate::{
        contract::execute,
        state::{Config, CONFIG},
    };

    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_hooks() {
        let mut deps = mock_dependencies();

        let messages = stake_hook_msgs(
            &deps.storage,
            Addr::unchecked("ekez"),
            "ekez-token".to_string(),
        )
        .unwrap();
        assert_eq!(messages.len(), 0);

        let messages = unstake_hook_msgs(
            &deps.storage,
            Addr::unchecked("ekez"),
            vec!["ekez-token".to_string()],
        )
        .unwrap();
        assert_eq!(messages.len(), 0);

        // Save a config for the execute messages we're testing.
        CONFIG
            .save(
                deps.as_mut().storage,
                &Config {
                    owner: Some(Addr::unchecked("ekez")),
                    manager: None,
                    nft_address: Addr::unchecked("ekez-token"),
                    unstaking_duration: None,
                },
            )
            .unwrap();

        let env = mock_env();
        let info = mock_info("ekez", &[]);

        execute(
            deps.as_mut(),
            env,
            info,
            crate::msg::ExecuteMsg::AddHook {
                addr: "ekez".to_string(),
            },
        )
        .unwrap();

        let messages = stake_hook_msgs(
            &deps.storage,
            Addr::unchecked("ekez"),
            "ekez-token".to_string(),
        )
        .unwrap();
        assert_eq!(messages.len(), 1);

        let messages = unstake_hook_msgs(
            &deps.storage,
            Addr::unchecked("ekez"),
            vec!["ekez-token".to_string()],
        )
        .unwrap();
        assert_eq!(messages.len(), 1);

        let env = mock_env();
        let info = mock_info("ekez", &[]);

        execute(
            deps.as_mut(),
            env,
            info,
            crate::msg::ExecuteMsg::RemoveHook {
                addr: "ekez".to_string(),
            },
        )
        .unwrap();

        let messages = stake_hook_msgs(
            &deps.storage,
            Addr::unchecked("ekez"),
            "ekez-token".to_string(),
        )
        .unwrap();
        assert_eq!(messages.len(), 0);

        let messages = unstake_hook_msgs(
            &deps.storage,
            Addr::unchecked("ekez"),
            vec!["ekez-token".to_string()],
        )
        .unwrap();
        assert_eq!(messages.len(), 0);
    }
}
