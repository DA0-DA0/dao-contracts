use cosmwasm_std::Uint128;
use cw_multi_test::next_block;
use cw_utils::Duration;

use crate::{
    state::MAX_CLAIMS,
    testing::{
        execute::{stake_nft, unstake_nfts},
        instantiate::instantiate_cw721_base,
        queries::query_voting_power,
    },
};

use super::{
    execute::mint_and_stake_nft, is_error, queries::query_total_and_voting_power, setup_test,
    CommonTest, CREATOR_ADDR,
};

/// Staking tokens has a one block delay before staked tokens are
/// reflected in voting power. Unstaking tokens has a one block delay
/// before the unstaking is reflected in voting power, yet you have
/// access to the NFT. If I immediately stake an unstaked NFT, my
/// voting power should not change.
#[test]
fn test_circular_stake() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(None);

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;

    app.update_block(next_block);

    let (total, voting) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(2));
    assert_eq!(voting, Uint128::new(2));

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["1", "2"])?;

    // Unchanged, one block delay.
    let (total, voting) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(2));
    assert_eq!(voting, Uint128::new(2));

    stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;

    // Unchanged.
    let (total, voting) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(2));
    assert_eq!(voting, Uint128::new(2));

    app.update_block(next_block);

    // Still unchanged.
    let (total, voting) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::new(2));
    assert_eq!(voting, Uint128::new(2));

    Ok(())
}

/// I can immediately unstake after staking even though voting powers
/// aren't updated until one block later. Voting power does not change
/// if I do this.
#[test]
fn test_immediate_unstake() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(None);

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;
    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "2")?;

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["1", "2"])?;

    app.update_block(next_block);

    let (total, voting) = query_total_and_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(total, Uint128::zero());
    assert_eq!(voting, Uint128::zero());

    Ok(())
}

/// I can not stake NFTs from a collection other than the one this has
/// been configured for.
#[test]
fn test_stake_wrong_nft() -> anyhow::Result<()> {
    let CommonTest {
        mut app, module, ..
    } = setup_test(None);
    let other_nft = instantiate_cw721_base(&mut app, CREATOR_ADDR, CREATOR_ADDR);

    let res = mint_and_stake_nft(&mut app, &other_nft, &module, CREATOR_ADDR, "1");
    is_error!(res => "Invalid token.");

    app.update_block(next_block);
    let voting = query_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(voting.power, Uint128::new(0));

    Ok(())
}

/// I can determine what my voting power _will_ be after staking by
/// asking for my voting power one block in the future.
#[test]
fn test_query_the_future() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(None);

    mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, "1")?;

    // Future voting power will be one under current conditions.
    let voting = query_voting_power(
        &app,
        &module,
        CREATOR_ADDR,
        Some(app.block_info().height + 100),
    )?;
    assert_eq!(voting.power, Uint128::new(1));

    // Current voting power is zero.
    let voting = query_voting_power(&app, &module, CREATOR_ADDR, None)?;
    assert_eq!(voting.power, Uint128::new(0));

    unstake_nfts(&mut app, &module, CREATOR_ADDR, &["1"])?;

    // Future voting power is now zero.
    let voting = query_voting_power(
        &app,
        &module,
        CREATOR_ADDR,
        Some(app.block_info().height + 100),
    )?;
    assert_eq!(voting.power, Uint128::zero());

    Ok(())
}

/// I can not unstake more than one NFT in a TX in order to bypass the
/// MAX_CLAIMS limit.
#[test]
fn test_bypass_max_claims() -> anyhow::Result<()> {
    let CommonTest {
        mut app,
        module,
        nft,
    } = setup_test(Some(Duration::Height(1)));
    let mut to_stake = vec![];
    for i in 1..(MAX_CLAIMS + 10) {
        let i_str = &i.to_string();
        mint_and_stake_nft(&mut app, &nft, &module, CREATOR_ADDR, i_str)?;
        if i < MAX_CLAIMS {
            // unstake MAX_CLAMS - 1 NFTs
            unstake_nfts(&mut app, &module, CREATOR_ADDR, &[i_str])?;
        } else {
            // push rest of NFT ids to vec
            to_stake.push(i_str.clone());
        }
    }
    let binding = to_stake.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let to_stake_slice: &[&str] = binding.as_slice();
    let res = unstake_nfts(&mut app, &module, CREATOR_ADDR, to_stake_slice);
    is_error!(res => "Too many outstanding claims. Claim some tokens before unstaking more.");
    Ok(())
}
