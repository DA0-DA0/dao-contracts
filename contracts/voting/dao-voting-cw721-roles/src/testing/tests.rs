use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::{App, Executor};
use dao_cw721_extensions::roles::MetadataExt;
use dao_testing::contracts::dao_voting_cw721_roles_contract;

use crate::{
    msg::{InstantiateMsg, NftContract, NftMintMsg},
    state::Config,
    testing::{
        execute::mint_nft,
        queries::{query_config, query_info, query_minter, query_total_power, query_voting_power},
    },
};

use super::{instantiate::instantiate_cw721_roles, setup_test, CommonTest, CREATOR_ADDR};

#[test]
fn test_info_query_works() -> anyhow::Result<()> {
    let CommonTest {
        app, module_addr, ..
    } = setup_test(vec![NftMintMsg {
        token_id: "1".to_string(),
        owner: CREATOR_ADDR.to_string(),
        token_uri: None,
        extension: MetadataExt {
            role: None,
            weight: 1,
        },
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
fn test_use_existing_nft_contract() {
    let mut app = App::default();
    let module_id = app.store_code(dao_voting_cw721_roles_contract());

    let (cw721_addr, _) = instantiate_cw721_roles(&mut app, CREATOR_ADDR, CREATOR_ADDR);
    let module_addr = app
        .instantiate_contract(
            module_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                nft_contract: NftContract::Existing {
                    address: cw721_addr.clone().to_string(),
                },
            },
            &[],
            "cw721_voting",
            None,
        )
        .unwrap();

    // Get total power
    let total = query_total_power(&app, &module_addr, None).unwrap();
    assert_eq!(total.power, Uint128::zero());

    // Creator mints themselves a new NFT
    mint_nft(&mut app, &cw721_addr, CREATOR_ADDR, CREATOR_ADDR, "1").unwrap();

    // Get voting power for creator
    let vp = query_voting_power(&app, &module_addr, CREATOR_ADDR, None).unwrap();
    assert_eq!(vp.power, Uint128::new(1));
}

#[test]
fn test_voting_queries() {
    let CommonTest {
        mut app,
        module_addr,
        ..
    } = setup_test(vec![NftMintMsg {
        token_id: "1".to_string(),
        owner: CREATOR_ADDR.to_string(),
        token_uri: None,
        extension: MetadataExt {
            role: Some("admin".to_string()),
            weight: 1,
        },
    }]);

    // Get config
    let config: Config = query_config(&app, &module_addr).unwrap();
    let cw721_addr = config.nft_address;

    // Get NFT minter. After the ownership handshake completes (setup_test
    // performs AcceptOwnership on behalf of the DAO), the minter / owner of
    // the cw721-roles contract is CREATOR_ADDR — i.e. the "DAO core" in this
    // test setup. The voting module no longer holds privileged rights.
    let minter = query_minter(&app, &cw721_addr.clone()).unwrap();
    assert_eq!(minter.minter, Some(CREATOR_ADDR.to_string()));

    // Get total power
    let total = query_total_power(&app, &module_addr, None).unwrap();
    assert_eq!(total.power, Uint128::new(1));

    // Get voting power for creator
    let vp = query_voting_power(&app, &module_addr, CREATOR_ADDR, None).unwrap();
    assert_eq!(vp.power, Uint128::new(1));

    // Mint a new NFT as the DAO (CREATOR_ADDR).
    mint_nft(&mut app, &cw721_addr, CREATOR_ADDR, CREATOR_ADDR, "2").unwrap();

    // Get total power
    let total = query_total_power(&app, &module_addr, None).unwrap();
    assert_eq!(total.power, Uint128::new(2));

    // Get voting power for creator
    let vp = query_voting_power(&app, &module_addr, CREATOR_ADDR, None).unwrap();
    assert_eq!(vp.power, Uint128::new(2));
}
