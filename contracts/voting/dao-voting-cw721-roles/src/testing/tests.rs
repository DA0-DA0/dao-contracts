use cosmwasm_std::Uint128;
use cw721_controllers::{NftClaim, NftClaimsResponse};
use cw_multi_test::next_block;
use cw_utils::Duration;
use dao_interface::Admin;

use crate::{
    state::Config,
    testing::{
        execute::{mint_nft, update_config},
        queries::{query_config, query_hooks, query_nft_owner, query_total_and_voting_power},
    },
};

use super::{
    execute::{add_hook, remove_hook},
    is_error,
    queries::{query_info, query_total_power, query_voting_power},
    setup_test, CommonTest, CREATOR_ADDR,
};

#[test]
fn test_info_query_works() -> anyhow::Result<()> {
    let CommonTest { app, module, .. } = setup_test(None, None);
    let info = query_info(&app, &module)?;
    assert_eq!(info.info.version, env!("CARGO_PKG_VERSION").to_string());
    Ok(())
}

// The owner may add and remove hooks.
#[test]
fn test_add_remove_hooks() -> anyhow::Result<()> {
    let CommonTest {
        mut app, module, ..
    } = setup_test(
        Some(Admin::Address {
            addr: CREATOR_ADDR.to_string(),
        }),
        None,
    );

    add_hook(&mut app, &module, CREATOR_ADDR, "meow")?;
    remove_hook(&mut app, &module, CREATOR_ADDR, "meow")?;

    add_hook(&mut app, &module, CREATOR_ADDR, "meow")?;

    let hooks = query_hooks(&app, &module)?;
    assert_eq!(hooks.hooks, vec!["meow".to_string()]);

    let res = add_hook(&mut app, &module, CREATOR_ADDR, "meow");
    is_error!(res => "Given address already registered as a hook");

    let res = remove_hook(&mut app, &module, CREATOR_ADDR, "blue");
    is_error!(res => "Given address not registered as a hook");

    let res = add_hook(&mut app, &module, "ekez", "evil");
    is_error!(res => "Only the owner of this contract my execute this message");

    Ok(())
}
