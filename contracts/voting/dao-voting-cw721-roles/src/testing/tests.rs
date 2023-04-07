use cosmwasm_std::Uint128;
use cw721_controllers::{NftClaim, NftClaimsResponse};
use cw721_roles::msg::MetadataExt;
use cw_multi_test::next_block;
use cw_utils::Duration;
use dao_interface::Admin;

use crate::{
    msg::NftMintMsg,
    state::Config,
    testing::{
        execute::mint_nft,
        queries::{
            query_config, query_hooks, query_info, query_nft_owner, query_total_and_voting_power,
            query_total_power, query_voting_power,
        },
    },
};

use super::{is_error, setup_test, CommonTest, CREATOR_ADDR};

#[test]
fn test_info_query_works() -> anyhow::Result<()> {
    let CommonTest {
        app, module_addr, ..
    } = setup_test(vec![NftMintMsg {
        token_id: "1".to_string(),
        owner: CREATOR_ADDR.to_string(),
        token_uri: None,
        extension: MetadataExt { weight: 1 },
    }]);
    let info = query_info(&app, &module_addr)?;
    assert_eq!(info.info.version, env!("CARGO_PKG_VERSION").to_string());
    Ok(())
}

#[test]
#[should_panic(expected = "New cw721-roles contract must be instantiated with at least one NFT")]
fn test_instantiate_no_roles_fails() {
    setup_test(vec![]);
}

#[test]
fn test_voting_queries() {
    let CommonTest {
        app,
        module_addr,
        cw721_addr,
        ..
    } = setup_test(vec![NftMintMsg {
        token_id: "1".to_string(),
        owner: CREATOR_ADDR.to_string(),
        token_uri: None,
        extension: MetadataExt { weight: 1 },
    }]);

    // Get total power
    let total = query_total_power(&app, &module_addr, None).unwrap();
    println!("{:?}", total);

    // Get voting power for creator
    let vp = query_voting_power(&app, &module_addr, CREATOR_ADDR, None).unwrap();
    println!("{:?}", vp);
}
