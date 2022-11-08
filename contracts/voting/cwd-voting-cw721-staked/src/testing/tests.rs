use cosmwasm_std::{Addr, Uint128};
use cw721_controllers::{NftClaim, NftClaimsResponse};
use cw_multi_test::{next_block, App, Executor};
use cw_utils::Duration;
use cwd_interface::Admin;
use cwd_testing::contracts::{cw721_base_contract, voting_cw721_staked_contract};

use crate::{
    msg::InstantiateMsg,
    state::{Config, MAX_CLAIMS},
    testing::{
        execute::{
            claim_nfts, mint_and_stake_nft, mint_nft, stake_nft, unstake_nfts, update_config,
        },
        queries::{query_config, query_hooks, query_nft_owner, query_total_and_voting_power},
    },
};

use super::{
    execute::{add_hook, remove_hook},
    queries::{query_claims, query_info, query_staked_nfts, query_total_power, query_voting_power},
    CREATOR_ADDR,
};

struct CommonTest {
    app: App,
    module: Addr,
    nft: Addr,
}
fn setup_test(owner: Option<Admin>, unstaking_duration: Option<Duration>) -> CommonTest {
    let mut app = App::default();
    let cw721_id = app.store_code(cw721_base_contract());
    let module_id = app.store_code(voting_cw721_staked_contract());

    let nft = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw721_base::InstantiateMsg {
                name: "bad kids".to_string(),
                symbol: "bad kids".to_string(),
                minter: CREATOR_ADDR.to_string(),
            },
            &[],
            "cw721_base".to_string(),
            None,
        )
        .unwrap();
    let module = app
        .instantiate_contract(
            module_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                owner,
                nft_address: nft.to_string(),
                unstaking_duration,
            },
            &[],
            "cw721_voting",
            None,
        )
        .unwrap();
    CommonTest { app, module, nft }
}

// I can stake tokens, voting power and total power is updated one
// block later.
#[test]
fn test_stake_tokens() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(None, None);

    let total_power = query_total_power(&app, &module, None)?;
    let voting_power = query_voting_power(&app, &module, CREATOR_ADDR, None)?;

    assert_eq!(total_power.power, Uint128::zero());
    assert_eq!(total_power.height, app.block_info().height);

    assert_eq!(voting_power.power, Uint128::zero());
    assert_eq!(voting_power.height, app.block_info().height);

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;

    // Voting powers are not updated until a block has passed.
    let (total, personal) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert!(total.is_zero());
    assert!(personal.is_zero());

    app.update_block(next_block);

    let (total, personal) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(1));
    assert_eq!(personal, Uint128::new(1));

    Ok(())
}

// I can unstake tokens. Unstaking more than one token at once
// works. I can not unstake a token more than once. I can not unstake
// another addresses' token. Voting power and total power is updated
// when I unstake.
#[test]
fn test_unstake_tokens_no_claims() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(None, None);

    let friend = "friend";

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "3")?;

    mint_nft(&mut app, &nft, CREATOR_ADDR, friend, "4")?;
    mint_nft(&mut app, &nft, CREATOR_ADDR, friend, "5")?;
    stake_nft(&mut app, &nft, &module, friend, "4")?;
    stake_nft(&mut app, &nft, &module, friend, "5")?;

    app.update_block(next_block);

    let (total, personal) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(5));
    assert_eq!(personal, Uint128::new(3));

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["1", "2"])?;

    // Voting power is updated when I unstake. Waits a block as it's a
    // snapshot map.
    let (total, personal) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(5));
    assert_eq!(personal, Uint128::new(3));
    app.update_block(next_block);
    let (total, personal) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(3));
    assert_eq!(personal, Uint128::new(1));

    // I can not unstake tokens I do not own. Anyhow can't figure out
    // how to downcast this error so we check for the expected string.
    let err = unstake_nfts(&mut app, &module, CREATOR_ADDR, &["4"]).unwrap_err();
    assert!(format!("{:?}", err)
        .contains("Can not unstake that which you have not staked (unstaking 4)"));

    let err = unstake_nfts(&mut app, &module, CREATOR_ADDR, &["5", "4"]).unwrap_err();
    assert!(format!("{:?}", err)
        .contains("Can not unstake that which you have not staked (unstaking 5)"));

    let err = unstake_nfts(&mut app, &module, CREATOR_ADDR, &["☯️", "4"]).unwrap_err();
    assert!(format!("{:?}", err)
        .contains("Can not unstake that which you have not staked (unstaking ☯️)"));

    // I can not unstake tokens more than once.
    let err = unstake_nfts(&mut app, &module, CREATOR_ADDR, &["1"]).unwrap_err();
    assert!(format!("{:?}", err)
        .contains("Can not unstake that which you have not staked (unstaking 1)"));

    Ok(())
}

// I can update the unstaking duration and the owner. Only the owner
// may do this. I can unset the owner. Updating the unstaking duration
// does not impact outstanding claims.
#[test]
fn test_update_config() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(Some(Admin::CoreModule {}), Some(Duration::Height(3)));

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["1"])?;

    let claims = query_claims(&app, &module, CREATOR_ADDR)?;
    assert_eq!(
        claims,
        NftClaimsResponse {
            nft_claims: vec![NftClaim {
                token_id: "1".to_string(),
                release_at: cw_utils::Expiration::AtHeight(app.block_info().height + 3)
            }]
        }
    );

    // Make friend the new owner.
    update_config(
        &mut app,
        &module,
        CREATOR_ADDR,
        Some("friend"),
        Some(Duration::Time(1)),
    )?;

    // Existing claims should remain unchanged.
    let claims = query_claims(&app, &module, CREATOR_ADDR)?;
    assert_eq!(
        claims,
        NftClaimsResponse {
            nft_claims: vec![NftClaim {
                token_id: "1".to_string(),
                release_at: cw_utils::Expiration::AtHeight(app.block_info().height + 3)
            }]
        }
    );

    // New claims should reflect the new unstaking duration. Old ones
    // should not.
    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["2"])?;
    let claims = query_claims(&app, &module, CREATOR_ADDR)?;
    assert_eq!(
        claims,
        NftClaimsResponse {
            nft_claims: vec![
                NftClaim {
                    token_id: "1".to_string(),
                    release_at: cw_utils::Expiration::AtHeight(app.block_info().height + 3)
                },
                NftClaim {
                    token_id: "2".to_string(),
                    release_at: Duration::Time(1).after(&app.block_info())
                }
            ]
        }
    );

    let info = app.block_info();
    app.update_block(|mut block| {
        block.height += 3;
        block.time = match Duration::Time(1).after(&info) {
            cw_utils::Expiration::AtTime(timestamp) => timestamp,
            _ => panic!("there should really be an easier way to do this"),
        }
    });

    // Do a claim for good measure.
    claim_nfts(&mut app, &module, CREATOR_ADDR)?;
    let claims = query_claims(&app, &module, CREATOR_ADDR)?;
    assert_eq!(claims, NftClaimsResponse { nft_claims: vec![] });

    // Creator can no longer do config updates.
    let err = update_config(
        &mut app,
        &module,
        CREATOR_ADDR,
        Some("friend"),
        Some(Duration::Time(1)),
    )
    .unwrap_err();
    assert!(format!("{:?}", err).contains("Unauthorized"));

    // Friend can still do config updates, and even remove themselves
    // as the owner.
    update_config(&mut app, &module, "friend", None, None)?;
    let config = query_config(&app, &module)?;
    assert_eq!(
        config,
        Config {
            owner: None,
            nft_address: nft,
            unstaking_duration: None
        }
    );

    // Friend has removed themselves.
    let err = update_config(
        &mut app,
        &module,
        "friend",
        Some("friend"),
        Some(Duration::Time(1)),
    )
    .unwrap_err();
    assert!(format!("{:?}", err).contains("Unauthorized"));

    Ok(())
}

// I can query my pending claims. Attempting to claim with nothing to
// claim results in an error. Attempting to claim with tokens to claim
// results in me owning those tokens.
#[test]
fn test_claims() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(Some(Admin::CoreModule {}), Some(Duration::Height(1)));

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "3")?;

    let claims = query_claims(&app, &module, CREATOR_ADDR)?;
    assert_eq!(claims.nft_claims, vec![]);

    let err = claim_nfts(&mut app, &module, CREATOR_ADDR).unwrap_err();
    assert!(format!("{:?}", err).contains("Nothing to claim"));

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["2"])?;

    let claims = query_claims(&app, &module, CREATOR_ADDR)?;
    assert_eq!(
        claims.nft_claims,
        vec![NftClaim {
            token_id: "2".to_string(),
            release_at: cw_utils::Expiration::AtHeight(app.block_info().height + 1)
        }]
    );

    // Claim now exists, but is not yet expired. Nothing to claim.
    let err = claim_nfts(&mut app, &module, CREATOR_ADDR).unwrap_err();
    assert!(format!("{:?}", err).contains("Nothing to claim"));

    app.update_block(next_block);
    claim_nfts(&mut app, &module, CREATOR_ADDR)?;

    let owner = query_nft_owner(&app, &nft, "2")?;
    assert_eq!(owner.owner, CREATOR_ADDR.to_string());

    Ok(())
}

// I can not have more than MAX_CLAIMS claims pending.
#[test]
fn test_max_claims() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(None, Some(Duration::Height(1)));

    for i in 0..MAX_CLAIMS {
        let i_str = &i.to_string();
        mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, i_str)?;
        unstake_nfts(&mut app, &module, CREATOR_ADDR, &[i_str])?;
    }

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "a")?;
    let err = unstake_nfts(&mut app, &module, CREATOR_ADDR, &["a"]).unwrap_err();
    assert!(format!("{:?}", err)
        .contains("Too many outstanding claims. Claim some tokens before unstaking more."));

    Ok(())
}

// I can list all of the currently staked NFTs for an address.
#[test]
fn test_list_staked_nfts() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(Some(Admin::CoreModule {}), Some(Duration::Height(1)));

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "3")?;

    let deardrie = "deardrie";
    mint_nft(&mut app, &nft, CREATOR_ADDR, deardrie, "4")?;
    mint_nft(&mut app, &nft, CREATOR_ADDR, deardrie, "5")?;

    let nfts = query_staked_nfts(&app, &module, deardrie, None, None)?;
    assert!(nfts.is_empty());

    stake_nft(&mut app, &nft, &module, deardrie, "4")?;
    stake_nft(&mut app, &nft, &module, deardrie, "5")?;

    let nfts = query_staked_nfts(&app, &module, deardrie, None, None)?;
    assert_eq!(nfts, vec!["4".to_string(), "5".to_string()]);

    let nfts = query_staked_nfts(&app, &module, CREATOR_ADDR, Some("1".to_string()), Some(0))?;
    assert!(nfts.is_empty());

    let nfts = query_staked_nfts(&app, &module, CREATOR_ADDR, Some("3".to_string()), None)?;
    assert!(nfts.is_empty());
    let nfts = query_staked_nfts(
        &app,
        &module,
        CREATOR_ADDR,
        Some("3".to_string()),
        Some(500),
    )?;
    assert!(nfts.is_empty());

    let nfts = query_staked_nfts(&app, &module, CREATOR_ADDR, Some("1".to_string()), Some(2))?;
    assert_eq!(nfts, vec!["2".to_string(), "3".to_string()]);

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["2"])?;
    let nfts = query_staked_nfts(&app, &module, CREATOR_ADDR, Some("1".to_string()), Some(2))?;
    assert_eq!(nfts, vec!["3".to_string()]);

    Ok(())
}

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

    let err = add_hook(&mut app, &module, CREATOR_ADDR, "meow").unwrap_err();
    assert!(format!("{:?}", err).contains("Given address already registered as a hook"));

    let err = remove_hook(&mut app, &module, CREATOR_ADDR, "blue").unwrap_err();
    assert!(format!("{:?}", err).contains("Given address not registered as a hook"));

    let err = add_hook(&mut app, &module, "ekez", "evil").unwrap_err();
    assert!(format!("{:?}", err).contains("Unauthorized"));

    Ok(())
}

// ----
// adversarial

// I can not unstake tokens that do not belong to me.

// I can not stake tokens from a collection that is not the configured
// one.

// What happens if I query for voting power for a block in the future?

// What happens if you repeatedly stake and unstake a NFT in a single
// TX. can you inflate voting power? can we make the snapshot map do
// something strange?
