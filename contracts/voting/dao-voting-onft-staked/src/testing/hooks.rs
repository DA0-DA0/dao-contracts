use cosmwasm_std::{
    testing::{message_info, mock_dependencies, mock_env},
    Addr,
};
use dao_hooks::nft_stake::{stake_nft_hook_msgs, unstake_nft_hook_msgs};

use crate::{
    contract::execute,
    state::{Config, CONFIG, DAO, HOOKS},
};

#[test]
fn test_hooks() {
    let mut deps = mock_dependencies();

    let messages = stake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        "ekez-token".to_string(),
    )
    .unwrap();
    assert_eq!(messages.len(), 0);

    let messages = unstake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        vec!["ekez-token".to_string()],
    )
    .unwrap();
    assert_eq!(messages.len(), 0);

    // Save a DAO address for the execute messages we're testing.
    DAO.save(
        deps.as_mut().storage,
        &Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
    )
    .unwrap();

    // Save a config for the execute messages we're testing.
    CONFIG
        .save(
            deps.as_mut().storage,
            &Config {
                onft_collection_id: "ekez-token".to_string(),
                unstaking_duration: None,
            },
        )
        .unwrap();

    let env = mock_env();
    let info = message_info(
        &Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        &[],
    );

    execute(
        deps.as_mut(),
        env,
        info,
        crate::msg::ExecuteMsg::AddHook {
            addr: "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(),
        },
    )
    .unwrap();

    let messages = stake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        "ekez-token".to_string(),
    )
    .unwrap();
    assert_eq!(messages.len(), 1);

    let messages = unstake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        vec!["ekez-token".to_string()],
    )
    .unwrap();
    assert_eq!(messages.len(), 1);

    let env = mock_env();
    let info = message_info(
        &Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        &[],
    );

    execute(
        deps.as_mut(),
        env,
        info,
        crate::msg::ExecuteMsg::RemoveHook {
            addr: "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(),
        },
    )
    .unwrap();

    let messages = stake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        "ekez-token".to_string(),
    )
    .unwrap();
    assert_eq!(messages.len(), 0);

    let messages = unstake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"),
        vec!["ekez-token".to_string()],
    )
    .unwrap();
    assert_eq!(messages.len(), 0);
}
