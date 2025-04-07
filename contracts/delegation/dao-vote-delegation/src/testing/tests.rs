use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    to_json_binary, Addr, Decimal, Empty, Uint128,
};
use cw_multi_test::{Contract, ContractWrapper};
use cw_utils::Duration;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};
use dao_interface::helpers::OptionalUpdate;
use dao_testing::{ADDR0, ADDR1, ADDR2, ADDR3, ADDR4};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION, DEFAULT_MAX_DELEGATIONS},
    ContractError,
};

use super::suite::{
    cw4::Cw4DaoVoteDelegationTestingSuite, cw721::Cw721DaoVoteDelegationTestingSuite,
    token::TokenDaoVoteDelegationTestingSuite,
};

pub fn dao_vote_delegation_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}

#[test]
fn test_simple() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new()
        .with_vp_cap_percent(Decimal::percent(50))
        .with_delegation_validity_blocks(10)
        .build();
    let dao = suite.dao.clone();

    // ensure set up correctly
    assert_eq!(
        suite.voting_power_hook_callers(None, None),
        vec![dao.x.group_addr.clone()]
    );
    assert_eq!(
        suite.proposal_modules(None, None),
        dao.proposal_modules
            .iter()
            .map(|p| p.1.clone())
            .collect::<Vec<_>>()
    );

    suite.assert_delegate_not_registered(ADDR0, None);

    // register ADDR0 as a delegate
    suite.register(ADDR0);
    suite.assert_delegates_count(1);

    // delegate 100% of addr1's voting power to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));

    // delegations take effect on the next block
    suite.advance_block();

    // ensure registered
    suite.assert_delegate_registered(ADDR0, None);
    suite.assert_delegate_registered(ADDR0, Some(suite.app.block_info().height));
    // historical check works
    suite.assert_delegate_not_registered(ADDR0, Some(suite.app.block_info().height - 1));

    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_delegation(ADDR1, ADDR0, Decimal::percent(100));
    suite.assert_delegate_total_delegated_vp(ADDR0, suite.members[1].weight);

    // propose a proposal
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR0, "test proposal 1", vec![]);

    // ensure delegation is correctly applied to proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[1].weight,
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[1].weight,
    );

    // set delegation to 50%
    suite.delegate(ADDR1, ADDR0, Decimal::percent(50));

    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegation(ADDR1, ADDR0, Decimal::percent(50));
    suite.assert_delegate_total_delegated_vp(ADDR0, suite.members[1].weight / 2);

    // propose another proposal
    let (_, id2, p2) = suite.propose_single_choice(&dao, ADDR2, "test proposal 2", vec![]);

    // ensure delegation is correctly applied to new proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        suite.members[1].weight / 2,
    );

    // ensure old delegation is still applied to old proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[1].weight,
    );

    // revoke delegation
    suite.undelegate(ADDR1, ADDR0);

    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegations_count(ADDR1, 0);
    suite.assert_delegate_total_delegated_vp(ADDR0, 0u128);

    // propose another proposal
    let (_, id3, p3) = suite.propose_single_choice(&dao, ADDR2, "test proposal 3", vec![]);

    // ensure delegation is removed from new proposal
    suite.assert_effective_udvp(ADDR0, &proposal_module, id3, p3.start_height, 0u128);
    suite.assert_total_udvp(ADDR0, &proposal_module, id3, p3.start_height, 0u128);

    // delegate 100% of every other member's voting power to ADDR0
    for member in suite.members.clone() {
        if member.addr != ADDR0 {
            suite.delegate(member.addr, ADDR0, Decimal::percent(100));
        }
    }

    // delegations take effect on the next block
    suite.advance_block();

    let total_vp_except_addr0 = suite
        .members
        .iter()
        .map(|m| if m.addr == ADDR0 { 0 } else { m.weight as u128 })
        .sum::<u128>();
    suite.assert_delegate_total_delegated_vp(ADDR0, total_vp_except_addr0);

    // propose another proposal
    let (_, id4, p4) = suite.propose_single_choice(&dao, ADDR0, "test proposal 4", vec![]);

    // ensure delegation is correctly applied to proposal and that VP cap is
    // applied correctly. effective should be 50% of total voting power, and
    // total should be everything that's delegated to ADDR0
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id4,
        p4.start_height,
        // VP cap is set to 50% of total voting power
        Uint128::from(suite.members.iter().map(|m| m.weight as u128).sum::<u128>())
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id4,
        p4.start_height,
        total_vp_except_addr0,
    );

    // advance 10 blocks to expire all delegations
    suite.advance_blocks(10);

    suite.assert_delegate_total_delegated_vp(ADDR0, 0u128);

    // propose another proposal
    let (_, id5, p5) = suite.propose_single_choice(&dao, ADDR0, "test proposal 5", vec![]);

    suite.assert_effective_udvp(ADDR0, &proposal_module, id5, p5.start_height, 0u128);
    suite.assert_total_udvp(ADDR0, &proposal_module, id5, p5.start_height, 0u128);

    // delegate 100% of every other member's voting power to ADDR0 again
    for member in suite.members.clone() {
        if member.addr != ADDR0 {
            suite.delegate(member.addr, ADDR0, Decimal::percent(100));
        }
    }

    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegate_total_delegated_vp(ADDR0, total_vp_except_addr0);

    suite.assert_delegate_registered(ADDR0, None);

    // unregister ADDR0 as a delegate
    suite.unregister(ADDR0);

    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegate_not_registered(ADDR0, None);
    suite.assert_delegates_count(0);

    // propose another proposal
    let (_, id6, p6) = suite.propose_single_choice(&dao, ADDR0, "test proposal 6", vec![]);

    suite.assert_effective_udvp(ADDR0, &proposal_module, id6, p6.start_height, 0u128);
    suite.assert_total_udvp(ADDR0, &proposal_module, id6, p6.start_height, 0u128);

    // ensure that ADDR1 has 1 delegation but 0 active delegations since their
    // delegate unregistered
    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_active_delegations_count(ADDR1, 0);
}

#[test]
fn test_voting_power_updates() {
    // Create a token-based DAO for testing
    let mut suite = TokenDaoVoteDelegationTestingSuite::new()
        .with_delegation_validity_blocks(20) // Set a shorter expiration period for testing
        .build();
    let dao = suite.dao.clone();

    // Register ADDR0 as a delegate
    suite.register(ADDR0);

    // Find the delegator's initial staked amount
    let delegator = ADDR1;
    let delegator_info = suite
        .members
        .iter()
        .find(|m| m.address == delegator)
        .expect("Delegator should exist in members");
    let initial_staked = delegator_info.amount;

    // Delegate 100% of ADDR1's voting power to ADDR0
    let delegation_block = suite.block().height;
    suite.delegate(delegator, ADDR0, Decimal::percent(100));

    // Delegations take effect on the next block
    suite.advance_block();

    // Verify the initial delegation amount
    suite.assert_delegate_total_delegated_vp(ADDR0, initial_staked);

    // CASE 1: INCREASE voting power by minting and staking more tokens
    let additional_stake = Uint128::from(500u128);
    suite.mint_and_stake(delegator, additional_stake);
    suite.advance_block();

    // Verify the delegated voting power increases correctly
    let expected_vp_after_increase = initial_staked + additional_stake;
    suite.assert_delegate_total_delegated_vp(ADDR0, expected_vp_after_increase);

    // CASE 2: DECREASE voting power by unstaking some tokens
    let unstake_amount = Uint128::from(200u128);
    suite.unstake(delegator, unstake_amount);
    suite.advance_block();

    // Verify the delegated voting power decreases correctly
    let expected_vp_after_decrease = expected_vp_after_increase - unstake_amount;
    suite.assert_delegate_total_delegated_vp(ADDR0, expected_vp_after_decrease);

    // CASE 3: Verify with proposal creation that delegation is correctly applied
    let (proposal_module, id, p) =
        suite.propose_single_choice(&dao, ADDR0, "test voting power updates", vec![]);

    // Verify delegate's effective voting power on the proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id,
        p.start_height,
        expected_vp_after_decrease,
    );

    // CASE 4: INCREASE voting power again
    let additional_stake_2 = Uint128::from(700u128);
    suite.mint_and_stake(delegator, additional_stake_2);
    suite.advance_block();

    // Verify the delegated voting power increases correctly
    let expected_vp_after_increase_2 = expected_vp_after_decrease + additional_stake_2;
    suite.assert_delegate_total_delegated_vp(ADDR0, expected_vp_after_increase_2);

    // Verify historical query still shows original voting power for earlier proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id,
        p.start_height,
        expected_vp_after_decrease,
    );

    // Create a new proposal to check the updated voting power
    let (proposal_module_2, id_2, p_2) =
        suite.propose_single_choice(&dao, ADDR0, "test updated voting power", vec![]);

    // Verify delegate's effective voting power on the new proposal
    suite.assert_effective_udvp(
        ADDR0,
        proposal_module_2,
        id_2,
        p_2.start_height,
        expected_vp_after_increase_2,
    );

    // CASE 5: Multiple operations test - unstake then stake in same block
    let unstake_amount_3 = Uint128::from(300u128);
    let stake_amount_3 = Uint128::from(100u128);

    suite.unstake(delegator, unstake_amount_3);
    suite.mint_and_stake(delegator, stake_amount_3);
    suite.advance_block();

    // Verify the delegated voting power is correctly updated after both operations
    let expected_final_vp = expected_vp_after_increase_2 - unstake_amount_3 + stake_amount_3;

    suite.assert_delegate_total_delegated_vp(ADDR0, expected_final_vp);

    // CASE 6: Test automatic expiration
    // Get original delegation expiration block
    let expiry_height = delegation_block + 20; // Based on .with_delegation_validity_blocks(20)

    // Advance to 1 block before expiration
    let blocks_to_advance = expiry_height - suite.block().height - 1;
    suite.advance_blocks(blocks_to_advance);

    // Delegation should still be active
    suite.assert_delegations_count(delegator, 1);
    suite.assert_delegate_total_delegated_vp(ADDR0, expected_final_vp);

    // Create a proposal just before expiration
    let (proposal_module_3, id_3, p_3) =
        suite.propose_single_choice(&dao, ADDR0, "test before expiration", vec![]);

    // Verify delegate's effective voting power on this pre-expiration proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module_3,
        id_3,
        p_3.start_height,
        expected_final_vp,
    );

    // Advance 1 more block so we hit expiration
    suite.advance_block();

    // Verify delegation is now expired
    suite.assert_delegations_count(delegator, 0);
    suite.assert_delegate_total_delegated_vp(ADDR0, 0u128);

    // Create a proposal after expiration
    let (proposal_module_4, id_4, p_4) =
        suite.propose_single_choice(&dao, ADDR0, "test after expiration", vec![]);

    // Verify delegate has no effective voting power on this post-expiration proposal
    suite.assert_effective_udvp(ADDR0, proposal_module_4, id_4, p_4.start_height, 0u128);

    // Verify that historical queries still show the correct voting power for proposals
    // created before expiration
    suite.assert_effective_udvp(
        ADDR0,
        proposal_module_3,
        id_3,
        p_3.start_height,
        expected_final_vp,
    );
}

#[test]
fn test_vp_cap_update() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new()
        .with_vp_cap_percent(Decimal::percent(50))
        .with_delegation_validity_blocks(10)
        .build();
    let dao = suite.dao.clone();

    // register ADDR0 as a delegate
    suite.register(ADDR0);

    // delegate 100% of every other member's voting power to ADDR0
    for member in suite.members.clone() {
        if member.addr != ADDR0 {
            suite.delegate(member.addr, ADDR0, Decimal::percent(100));
        }
    }

    // delegations take effect on the next block
    suite.advance_block();

    let first_block_height = suite.block().height;

    let total_vp_except_addr0 = suite
        .members
        .iter()
        .map(|m| if m.addr == ADDR0 { 0 } else { m.weight as u128 })
        .sum::<u128>();
    suite.assert_delegate_total_delegated_vp(ADDR0, total_vp_except_addr0);

    // propose a proposal
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // ensure delegation is correctly applied to proposal and that VP cap is
    // applied correctly. effective should be 50% of total voting power, and
    // total should be everything that's delegated to ADDR0
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        // VP cap is set to 50% of total voting power
        Uint128::from(suite.members.iter().map(|m| m.weight as u128).sum::<u128>())
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        total_vp_except_addr0,
    );

    suite.assert_voting_power_cap(None, Some(Decimal::percent(50)));
    suite.assert_voting_power_cap(Some(suite.block().height), Some(Decimal::percent(50)));
    suite.assert_voting_power_cap(Some(first_block_height), Some(Decimal::percent(50)));

    // change VP cap to 30% of total
    suite.update_vp_cap_percent(Some(Decimal::percent(30)));
    // updates take effect on the next block
    suite.advance_block();

    suite.assert_voting_power_cap(None, Some(Decimal::percent(30)));
    suite.assert_voting_power_cap(Some(suite.block().height), Some(Decimal::percent(30)));

    // historical query still works
    suite.assert_voting_power_cap(Some(first_block_height), Some(Decimal::percent(50)));

    // propose another proposal
    let (_, id2, p2) = suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // ensure delegation is correctly applied to proposal and that VP cap is
    // applied correctly. effective should be 30% of total voting power, and
    // total should still be everything that's delegated to ADDR0
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        // VP cap is set to 30% of total voting power
        Uint128::from(suite.members.iter().map(|m| m.weight as u128).sum::<u128>())
            .mul_floor(Decimal::percent(30)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        total_vp_except_addr0,
    );

    // old proposal should still use old VP cap
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        // VP cap is set to 50% of total voting power
        Uint128::from(suite.members.iter().map(|m| m.weight as u128).sum::<u128>())
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        total_vp_except_addr0,
    );

    // remove VP cap
    suite.update_vp_cap_percent(None);
    // updates take effect on the next block
    suite.advance_block();
    suite.assert_voting_power_cap(None, None);
    suite.assert_voting_power_cap(Some(suite.block().height), None);

    // historical query still works
    suite.assert_voting_power_cap(Some(first_block_height), Some(Decimal::percent(50)));

    // propose another proposal
    let (_, id3, p3) = suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // effective should now be equal to total since there is no cap
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id3,
        p3.start_height,
        total_vp_except_addr0,
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id3,
        p3.start_height,
        total_vp_except_addr0,
    );

    // old proposals should still use old VP caps
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        // VP cap is set to 30% of total voting power
        Uint128::from(suite.members.iter().map(|m| m.weight as u128).sum::<u128>())
            .mul_floor(Decimal::percent(30)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        total_vp_except_addr0,
    );
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        // VP cap is set to 50% of total voting power
        Uint128::from(suite.members.iter().map(|m| m.weight as u128).sum::<u128>())
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        total_vp_except_addr0,
    );
}

#[test]
fn test_expiration_update() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new()
        .with_delegation_validity_blocks(10)
        .build();

    // register ADDR0 as a delegate
    suite.register(ADDR0);

    // delegate to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_delegation(ADDR1, ADDR0, Decimal::percent(100));
    suite.assert_delegate_total_delegated_vp(ADDR0, suite.members[1].weight);

    // update delegation validity blocks to 50
    suite.update_delegation_validity_blocks(Some(50));

    // move 10 blocks into the future
    suite.advance_blocks(10);

    // delegation should be expired after 10 blocks since update happened after
    suite.assert_delegations_count(ADDR1, 0);
    suite.assert_delegate_total_delegated_vp(ADDR0, 0u128);

    // delegate to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    // delegations take effect on the next block
    suite.advance_block();

    // move 10 blocks into the future
    suite.advance_blocks(10);

    // delegation should still be active
    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_delegation(ADDR1, ADDR0, Decimal::percent(100));
    suite.assert_delegate_total_delegated_vp(ADDR0, suite.members[1].weight);

    // move 40 blocks into the future
    suite.advance_blocks(40);

    // delegation should be expired
    suite.assert_delegations_count(ADDR1, 0);
    suite.assert_delegate_total_delegated_vp(ADDR0, 0u128);

    suite.advance_block();

    // remove expiration
    suite.update_delegation_validity_blocks(None);

    // delegate to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    // delegations take effect on the next block
    suite.advance_block();

    // move 10 blocks into the future
    suite.advance_blocks(10);

    // delegation should still be active
    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_delegation(ADDR1, ADDR0, Decimal::percent(100));
    suite.assert_delegate_total_delegated_vp(ADDR0, suite.members[1].weight);

    // move 100 blocks into the future
    suite.advance_blocks(100);

    // delegation should still be active
    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_delegation(ADDR1, ADDR0, Decimal::percent(100));
    suite.assert_delegate_total_delegated_vp(ADDR0, suite.members[1].weight);
}

#[test]
fn test_max_delegations() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new()
        .with_max_delegations(2)
        .build();

    // register 3 delegates
    suite.register(ADDR0);
    suite.register(ADDR1);
    suite.register(ADDR2);

    // delegate to ADDR0 and ADDR1
    suite.delegate(ADDR3, ADDR0, Decimal::percent(10));
    suite.delegate(ADDR3, ADDR1, Decimal::percent(10));
    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegations_count(ADDR3, 2);

    // update delegation
    suite.delegate(ADDR3, ADDR0, Decimal::percent(20));
    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegations_count(ADDR3, 2);

    // try to delegate to ADDR2
    let err = suite.delegate_error(ADDR3, ADDR2, Decimal::percent(10));
    assert_eq!(
        err,
        ContractError::MaxDelegationsReached { max: 2, current: 2 }
    );

    suite.assert_delegations_count(ADDR3, 2);

    // lower max delegations to 1
    suite.update_max_delegations(1);
    suite.assert_max_delegations(1);

    // try to delegate to ADDR2
    let err = suite.delegate_error(ADDR3, ADDR2, Decimal::percent(10));
    assert_eq!(
        err,
        ContractError::MaxDelegationsReached { max: 1, current: 2 }
    );

    // try to update existing delegation
    let err = suite.delegate_error(ADDR3, ADDR1, Decimal::percent(20));
    assert_eq!(
        err,
        ContractError::MaxDelegationsReached { max: 1, current: 2 }
    );

    // remove a delegation
    suite.undelegate(ADDR3, ADDR0);

    // now update existing delegation
    suite.delegate(ADDR3, ADDR1, Decimal::percent(25));

    // delegations take effect on the next block
    suite.advance_block();

    suite.assert_delegations_count(ADDR3, 1);
    suite.assert_delegation(ADDR3, ADDR1, Decimal::percent(25));
}

#[test]
fn test_different_daos_delegations_query() {
    let mut token_dao_suite = TokenDaoVoteDelegationTestingSuite::new().build();

    let mut cw4_dao_suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    let mut cw721_dao_suite = Cw721DaoVoteDelegationTestingSuite::new().build();

    let token_dao_block = token_dao_suite.block().height;
    let cw4_dao_block = cw4_dao_suite.block().height;
    let cw721_dao_block = cw721_dao_suite.block().height;

    assert_eq!(token_dao_block, 1);
    assert_eq!(cw4_dao_block, 1);
    assert_eq!(cw721_dao_block, 1);

    // register a delegate to all daos
    cw4_dao_suite.register(ADDR0);
    token_dao_suite.register(ADDR0);
    cw721_dao_suite.register(ADDR0);

    // have a member delegate to the delegate in all daos
    cw4_dao_suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    token_dao_suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    cw721_dao_suite.delegate(ADDR1, ADDR0, Decimal::percent(100));

    // skip some blocks
    cw4_dao_suite.advance_blocks(2);
    token_dao_suite.advance_blocks(2);
    cw721_dao_suite.advance_blocks(2);

    let cw4_delegations = cw4_dao_suite
        .delegations(ADDR1.to_string(), Some(1), None, None)
        .delegations;
    let token_delegations = token_dao_suite
        .delegations(ADDR1.to_string(), Some(1), None, None)
        .delegations;
    let cw721_delegations = cw721_dao_suite
        .delegations(ADDR1.to_string(), Some(1), None, None)
        .delegations;

    assert!(cw4_delegations.is_empty());
    assert!(token_delegations.is_empty());
    assert!(cw721_delegations.is_empty());

    let cw4_delegations = cw4_dao_suite
        .delegations(ADDR1.to_string(), Some(2), None, None)
        .delegations;
    let token_delegations = token_dao_suite
        .delegations(ADDR1.to_string(), Some(2), None, None)
        .delegations;
    let cw721_delegations = cw721_dao_suite
        .delegations(ADDR1.to_string(), Some(2), None, None)
        .delegations;

    assert_eq!(cw4_delegations.len(), 1);
    assert_eq!(token_delegations.len(), 1);
    assert_eq!(cw721_delegations.len(), 1);
}

#[test]
fn test_token_dao_update_expiration_period() {
    let mut suite = TokenDaoVoteDelegationTestingSuite::new()
        .with_delegation_validity_blocks(5)
        .build();

    // register a delegate
    suite.register(ADDR0);

    // delegate to ADDR0 at block 1, expiring at block 1 + 5 = 6
    suite.delegate(ADDR3, ADDR0, Decimal::percent(100));

    // block height: 2
    suite.advance_block();
    suite.assert_delegations_count(ADDR3, 1);

    // block height: 3
    suite.advance_block();
    suite.assert_delegations_count(ADDR3, 1);

    // at block 5 the delegation should still be valid
    let block_5_delegations = suite.delegations(ADDR3, Some(5), None, None);
    assert_eq!(block_5_delegations.delegations.len(), 1);

    // at block 6 the delegation should be expired
    let block_6_delegations = suite.delegations(ADDR3, Some(6), None, None);
    assert_eq!(block_6_delegations.delegations.len(), 0);

    // dao updates the config and shortens the delegation validity blocks to 2
    suite.update_delegation_validity_blocks(Some(2));

    // delegate to ADDR0 at block 3. subject to new delegation expiration cfg,
    // this should expire at block 3 + 2 = 5
    suite.delegate(ADDR3, ADDR0, Decimal::percent(100));

    suite.assert_delegations_count(ADDR3, 1);

    // advance to block 4, there should still be an active delegation
    suite.advance_block();
    suite.assert_delegations_count(ADDR3, 1);

    // at blocks 5, 6... the delegation should be expired
    suite.advance_block();
    suite.assert_delegations_count(ADDR3, 0);

    suite.advance_block();
    suite.assert_delegations_count(ADDR3, 0);
}

#[test]
fn test_update_hook_callers() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let dao = suite.dao.clone();

    // ensure setup correctly
    assert_eq!(
        suite.voting_power_hook_callers(None, None),
        vec![dao.x.group_addr.clone()]
    );
    assert_eq!(
        suite.proposal_modules(None, None),
        dao.proposal_modules
            .iter()
            .map(|p| p.1.clone())
            .collect::<Vec<_>>()
    );

    // add another contract as a voting power hook caller
    suite.update_voting_power_hook_callers(Some(vec!["addr".to_string()]), None);

    assert_eq!(
        suite.voting_power_hook_callers(None, None),
        vec![Addr::unchecked("addr"), dao.x.group_addr.clone()]
    );

    // add another proposal module to the DAO
    let proposal_sudo_code_id = suite.proposal_sudo_id;
    suite.execute_smart_ok(
        &dao.core_addr,
        &dao.core_addr,
        &dao_interface::msg::ExecuteMsg::UpdateProposalModules {
            to_add: vec![dao_interface::state::ModuleInstantiateInfo {
                code_id: proposal_sudo_code_id,
                msg: to_json_binary(&dao_proposal_sudo::msg::InstantiateMsg {
                    root: "root".to_string(),
                })
                .unwrap(),
                admin: None,
                label: "sudo".to_string(),
                funds: None,
                salt: None,
            }],
            to_disable: vec![],
        },
        &[],
    );

    // sync proposal modules
    suite.sync_proposal_modules(None, None);

    // ensure new proposal module is synced
    assert_eq!(
        suite.proposal_modules(None, None).len(),
        dao.proposal_modules.len() + 1
    );
}

#[test]
fn test_vote_with_override() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let dao = suite.dao.clone();

    // register ADDR0 and ADDR3 as delegates
    suite.register(ADDR0);
    suite.register(ADDR3);

    // delegate all of ADDR1's and half of ADDR2's voting power to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    suite.delegate(ADDR2, ADDR0, Decimal::percent(50));
    // delegate all of ADDR4's voting power to ADDR3
    suite.delegate(ADDR4, ADDR3, Decimal::percent(100));

    // delegations take effect on the next block
    suite.advance_block();

    // ensure delegations are correctly applied
    suite.assert_delegations_count(ADDR1, 1);
    suite.assert_delegations_count(ADDR2, 1);
    suite.assert_delegations_count(ADDR4, 1);

    // propose a proposal
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR2, "test proposal", vec![]);

    // ADDR0 has 100% of ADDR1's voting power and 50% of ADDR2's voting power
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[1].weight + suite.members[2].weight / 2,
    );
    // ADDR3 has 100% of ADDR4's voting power
    suite.assert_effective_udvp(
        ADDR3,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[4].weight,
    );

    // delegate ADDR0 votes on proposal
    suite.vote_single_choice(&dao, ADDR0, id1, dao_voting::voting::Vote::Yes);

    // ADDR0 votes with own voting power, 100% of ADDR1's voting power, and 50%
    // of ADDR2's voting power
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        suite.members[0].weight + suite.members[1].weight + suite.members[2].weight / 2,
    );

    // ensure ADDR0 delegate will lose all of ADDR1's 100% delegated voting
    // power on this proposal if they override their vote
    suite.assert_delegated_vote_power_reduction(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[1].weight,
        suite.members[1].weight,
    );

    // ADDR1 overrides ADDR0's vote
    suite.vote_single_choice(&dao, ADDR1, id1, dao_voting::voting::Vote::No);
    // ADDR0's unvoted delegated voting power should no longer include ADDR1's
    // voting power on this proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[2].weight / 2,
    );
    // vote counts should change to reflect removed (overridden) delegate vote
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        suite.members[0].weight + suite.members[2].weight / 2,
    );
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::No,
        suite.members[1].weight,
    );

    // ensure ADDR3 delegate will lose all of ADDR4's 100% delegated voting
    // power on this proposal if they override their vote
    suite.assert_delegated_vote_power_reduction(
        ADDR3,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[4].weight,
        suite.members[4].weight,
    );

    // ADDR4 votes before their delegate ADDR3 does
    suite.vote_single_choice(&dao, ADDR4, id1, dao_voting::voting::Vote::Abstain);
    // ADDR3 unvoted delegated voting power should not include ADDR4's voting
    // power anymore, meaning it's zero
    suite.assert_effective_udvp(ADDR3, &proposal_module, id1, p1.start_height, 0u128);
    // abstain should count ADDR4's voting power
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Abstain,
        suite.members[4].weight,
    );

    // ADDR3 votes
    suite.vote_single_choice(&dao, ADDR3, id1, dao_voting::voting::Vote::No);
    // no votes should only include ADDR3's voting power (and ADDR1 from
    // before). ADDR4's delegated VP should not be counted here since they
    // already voted
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::No,
        suite.members[1].weight + suite.members[3].weight,
    );

    // ensure ADDR0 delegate will lose all of ADDR2's 50% delegated voting power
    // on this proposal if they override their vote
    suite.assert_delegated_vote_power_reduction(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[2].weight / 2,
        suite.members[2].weight / 2,
    );

    // ADDR2 overrides ADDR0's vote
    suite.vote_single_choice(&dao, ADDR2, id1, dao_voting::voting::Vote::Yes);
    // UDVP should now be zero for ADDR0 since all of their delegates overrode
    // their votes.
    suite.assert_effective_udvp(ADDR0, &proposal_module, id1, p1.start_height, 0u128);
    // now yes should count all of ADDR0 and ADDR2's voting power
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        suite.members[0].weight + suite.members[2].weight,
    );
}

#[test]
fn test_vote_override_with_cap() {
    let vp_cap_percent = Decimal::percent(20);
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new()
        .with_vp_cap_percent(vp_cap_percent)
        .build();
    let dao = suite.dao.clone();

    let delegation_cap = suite
        .total_voting_power(&dao.core_addr)
        .mul_floor(vp_cap_percent);

    suite.register(ADDR2);

    // delegate all of ADDR3's and ADDR0's voting power to ADDR2
    suite.delegate(ADDR3, ADDR2, Decimal::percent(100));
    suite.delegate(ADDR0, ADDR2, Decimal::percent(100));
    suite.advance_block();

    let total_udvp = suite.members[3].weight + suite.members[0].weight;

    // propose a proposal
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR2, "test proposal", vec![]);

    // delegate casts vote with all their VP and delegator's VP capped at 20%
    suite.vote_single_choice(&dao, ADDR2, id1, dao_voting::voting::Vote::Yes);
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        Uint128::from(suite.members[2].weight) + delegation_cap,
    );

    // ADDR2 has 100% of ADDR3's and ADDR0's voting power
    suite.assert_total_udvp(ADDR2, &proposal_module, id1, p1.start_height, total_udvp);
    // but cap is less so effective UDVP is capped
    assert!(delegation_cap < total_udvp.into());
    suite.assert_effective_udvp(
        ADDR2,
        &proposal_module,
        id1,
        p1.start_height,
        delegation_cap,
    );

    // if ADDR0 were to override the delegate's vote, the effective UDVP should
    // stay the same, since it's capped below the total UDVP and ADDR0's voting
    // power is only 1
    suite.assert_delegated_vote_power_reduction(
        ADDR2,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[0].weight,
        0u128,
    );

    // if ADDR3 were to override the delegate's vote, the effective UDVP will
    // reduce by part of ADDR3's VP since the cap is applied: the new effective
    // UDVP is total UDVP minus ADDR3's VP, so the reduction is the difference
    // between the cap (which is the current effective UDVP) and the new value
    let new_effective_udvp = Uint128::from(total_udvp - suite.members[3].weight);
    suite.assert_delegated_vote_power_reduction(
        ADDR2,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[3].weight,
        delegation_cap - new_effective_udvp,
    );

    // ADDR3 cast a vote, bringing the effective UDVP below the cap
    suite.vote_single_choice(&dao, ADDR3, id1, dao_voting::voting::Vote::No);
    // effective UDVP should be reduced by the full amount of ADDR3's VP
    suite.assert_effective_udvp(
        ADDR2,
        &proposal_module,
        id1,
        p1.start_height,
        new_effective_udvp,
    );
    // since effective UDVP is below the cap, it should match the total UDVP
    suite.assert_total_udvp(
        ADDR2,
        &proposal_module,
        id1,
        p1.start_height,
        new_effective_udvp,
    );

    // since ADDR3 voted and reduced ADDR2's effective UDVP, the vote counts
    // should reflect that. YES should have ADDR2's individual VP and new
    // effective UDVP after the ADDR3 reduction, and NO should have ADDR3's VP
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        Uint128::from(suite.members[2].weight) + new_effective_udvp,
    );
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::No,
        suite.members[3].weight,
    );

    // now if ADDR0 were to override the delegate's vote, the effective UDVP
    // should be reduced by the full amount of ADDR0's VP since the effective
    // UDVP is already below the cap
    suite.assert_delegated_vote_power_reduction(
        ADDR2,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[0].weight,
        suite.members[0].weight,
    );

    // ADDR0 votes yes in agreement with the delegate's vote
    suite.vote_single_choice(&dao, ADDR0, id1, dao_voting::voting::Vote::Yes);
    // UDVP should be reduced entirely to 0 since both delegators voted
    suite.assert_effective_udvp(ADDR2, &proposal_module, id1, p1.start_height, 0u128);
    suite.assert_total_udvp(ADDR2, &proposal_module, id1, p1.start_height, 0u128);
    // vote counts should not change since the delegator's vote matched the
    // delegate's vote
    assert_eq!(new_effective_udvp, Uint128::from(suite.members[0].weight));
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        suite.members[2].weight + suite.members[0].weight,
    );
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::No,
        suite.members[3].weight,
    );
}

#[test]
fn test_cw721_hook_handling() {
    let mut suite = Cw721DaoVoteDelegationTestingSuite::new().build();
    let dao = suite.dao.clone();

    let tid_3 = suite.members[3].token_id.to_string();

    // register ADDR0 as delegate
    suite.register(ADDR0);

    // addr3 and addr4 delegate to addr0
    suite.delegate(ADDR3, ADDR0, Decimal::percent(100));
    suite.delegate(ADDR4, ADDR0, Decimal::percent(100));

    suite.advance_block();

    // first prop
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR3, "test proposal", vec![]);

    suite.assert_effective_udvp(ADDR0, proposal_module, id1, p1.start_height, 2u128);

    // addr3 decides to unstake their cw721, which should decrease the effective
    // udvp of addr0
    suite.unstake(ADDR3, &tid_3);

    suite.advance_block();
    suite.advance_block();

    // second prop
    let (proposal_module, id2, p2) =
        suite.propose_single_choice(&dao, ADDR0, "test proposal 2", vec![]);

    // assert that addr0's effective udvp is now 1
    suite.assert_effective_udvp(ADDR0, proposal_module, id2, p2.start_height, 1u128);

    // addr3 stakes again, having not undelegated their voting power from addr0.
    // this should increase the effective udvp of addr0.
    suite.stake(ADDR3, &tid_3);

    suite.advance_block();
    suite.advance_block();

    // propose a proposal
    let (proposal_module, id3, p3) =
        suite.propose_single_choice(&dao, ADDR0, "test proposal 3", vec![]);

    // assert that addr3's stake is now reflected in addr0's effective udvp again
    suite.assert_effective_udvp(ADDR0, proposal_module, id3, p3.start_height, 2u128);
}

#[test]
fn test_overrideable_vote_doesnt_end_proposal_early() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let dao = suite.dao.clone();

    // register ADDR0 and ADDR1 as delegates
    suite.register(ADDR0);
    suite.register(ADDR1);

    // delegate all of ADDR2's and ADDR3's voting power to ADDR0
    suite.delegate(ADDR2, ADDR0, Decimal::percent(100));
    suite.delegate(ADDR3, ADDR0, Decimal::percent(100));
    // delegate all of ADDR4's voting power to ADDR1
    suite.delegate(ADDR4, ADDR1, Decimal::percent(100));

    // delegations take effect on the next block
    suite.advance_block();

    // ensure delegations are correctly applied
    suite.assert_delegations_count(ADDR2, 1);
    suite.assert_delegations_count(ADDR3, 1);
    suite.assert_delegations_count(ADDR4, 1);

    // propose a proposal
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR2, "test proposal", vec![]);

    // ADDR0 has 100% of ADDR2's voting power and 100% of ADDR3's voting power
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[2].weight + suite.members[3].weight,
    );
    // ADDR1 has 100% of ADDR4's voting power
    suite.assert_effective_udvp(
        ADDR1,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[4].weight,
    );

    // delegate ADDR0 votes on proposal
    suite.vote_single_choice(&dao, ADDR0, id1, dao_voting::voting::Vote::Yes);

    // proposal should not pass early, even though sufficient voting power has
    // voted for the configured threshold/quorum, because the delegators can
    // override the delegate's vote and change the outcome
    suite.assert_single_choice_status(&proposal_module, id1, dao_voting::status::Status::Open);

    // ADDR0 votes with own voting power, 100% of ADDR2's voting power, and 100%
    // of ADDR3's voting power
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        suite.members[0].weight + suite.members[2].weight + suite.members[3].weight,
    );

    // ADDR2 overrides ADDR0's vote
    suite.vote_single_choice(&dao, ADDR2, id1, dao_voting::voting::Vote::No);
    // ADDR0's unvoted delegated voting power should no longer include ADDR2's
    // voting power on this proposal
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        suite.members[3].weight,
    );
    // vote counts should change to reflect removed (overridden) delegate vote
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::Yes,
        suite.members[0].weight + suite.members[3].weight,
    );
    suite.assert_single_choice_votes_count(
        &proposal_module,
        id1,
        dao_voting::voting::Vote::No,
        suite.members[2].weight,
    );

    // proposal should still be open since only ADDR0's personal voting power
    // (1) and ADDR2's voting power (3) has been counted as definitive votes.
    // The remaining 6 voting power has either not been used to cast a vote or
    // is defaulting to a delegate's vote but can still be overridden.
    suite.assert_single_choice_status(&proposal_module, id1, dao_voting::status::Status::Open);

    // delegator ADDR3 votes, adding their 3 VP to ADDR2's 3 VP, meaning the
    // outcome is now determined to be No.
    suite.vote_single_choice(&dao, ADDR3, id1, dao_voting::voting::Vote::No);

    // proposal should be rejected since the outcome is now determined to be No
    suite.assert_single_choice_status(&proposal_module, id1, dao_voting::status::Status::Rejected);
}

#[test]
fn test_allow_register_after_unregister_same_block() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.unregister(ADDR0);
    suite.register(ADDR0);

    // ensure registered
    suite.advance_block();
    suite.assert_delegate_registered(ADDR0, None);
}

#[test]
fn test_allow_register_after_unregister_next_block() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.advance_block();
    suite.unregister(ADDR0);
    suite.advance_block();
    suite.register(ADDR0);

    // ensure registered
    suite.advance_block();
    suite.assert_delegate_registered(ADDR0, None);
}

#[test]
#[should_panic(expected = "invalid delegation validity blocks: provided 1, minimum 2")]
fn test_validate_delegation_validity_blocks() {
    Cw4DaoVoteDelegationTestingSuite::new()
        .with_delegation_validity_blocks(1)
        .build();
}

#[test]
#[should_panic(expected = "invalid delegation validity blocks: provided 1, minimum 2")]
fn test_validate_delegation_validity_blocks_update() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.update_delegation_validity_blocks(Some(1));
}

#[test]
#[should_panic(
    expected = "invalid voting power percent: must be greater than 0 and less than or equal to 1"
)]
fn test_validate_vp_cap_percent() {
    Cw4DaoVoteDelegationTestingSuite::new()
        .with_vp_cap_percent(Decimal::percent(101))
        .build();
}

#[test]
#[should_panic(
    expected = "invalid voting power percent: must be greater than 0 and less than or equal to 1"
)]
fn test_validate_vp_cap_percent_update() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.update_vp_cap_percent(Some(Decimal::percent(101)));
}

#[test]
fn test_max_delegations_config() {
    // instantiate with nothing, should set default
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.assert_max_delegations(DEFAULT_MAX_DELEGATIONS);

    // update config
    suite.update_max_delegations(75);
    suite.assert_max_delegations(75);

    // instantiate with a value set
    let suite = Cw4DaoVoteDelegationTestingSuite::new()
        .with_max_delegations(100)
        .build();

    suite.assert_max_delegations(100);
}

#[test]
#[should_panic(expected = "delegate already registered")]
fn test_no_double_register() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.register(ADDR0);
}

#[test]
#[should_panic(expected = "no voting power")]
fn test_no_vp_register() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register("non_member");
}

#[test]
#[should_panic(expected = "cannot register as a delegate with existing delegations")]
fn test_cannot_register_with_delegations_same_block() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    suite.register(ADDR1);
}

#[test]
#[should_panic(expected = "cannot register as a delegate with existing delegations")]
fn test_cannot_register_with_delegations_next_block() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    suite.advance_block();
    suite.register(ADDR1);
}

#[test]
#[should_panic(expected = "delegate not registered")]
fn test_cannot_unregister_unregistered() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.unregister(ADDR0);
}

#[test]
#[should_panic(
    expected = "invalid voting power percent: must be greater than 0 and less than or equal to 1"
)]
fn test_cannot_delegate_zero_percent() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::zero());
}

#[test]
#[should_panic(
    expected = "invalid voting power percent: must be greater than 0 and less than or equal to 1"
)]
fn test_cannot_delegate_more_than_100_percent() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::percent(101));
}

#[test]
#[should_panic(expected = "delegates cannot delegate to others")]
fn test_delegates_cannot_delegate() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.register(ADDR1);
    suite.delegate(ADDR0, ADDR1, Decimal::percent(100));
}

#[test]
#[should_panic(expected = "delegate not registered")]
fn test_cannot_delegate_unregistered() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.delegate(ADDR0, ADDR1, Decimal::percent(100));
}

#[test]
#[should_panic(expected = "no voting power")]
fn test_cannot_delegate_no_vp() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate("not_member", ADDR0, Decimal::percent(100));
}

#[test]
#[should_panic(expected = "cannot delegate more than 100% (current: 50%, attempt: 101%)")]
fn test_cannot_delegate_more_than_100() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.register(ADDR1);
    suite.delegate(ADDR2, ADDR0, Decimal::percent(50));
    suite.delegate(ADDR2, ADDR1, Decimal::percent(51));
}

#[test]
#[should_panic(expected = "delegation does not exist")]
fn test_cannot_undelegate_nonexistent() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.undelegate(ADDR0, ADDR1);
}

#[test]
fn test_delegate_undelegate_same_block() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    suite.undelegate(ADDR1, ADDR0);
}

#[test]
#[should_panic(expected = "delegation does not exist")]
fn test_cannot_undelegate_twice() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    suite.undelegate(ADDR1, ADDR0);
    suite.undelegate(ADDR1, ADDR0);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_unauthorized_update_voting_power_hook_callers() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let delegation_addr = suite.delegation_addr.clone();

    suite.execute_smart_ok(
        "no_one",
        &delegation_addr,
        &crate::msg::ExecuteMsg::UpdateVotingPowerHookCallers {
            add: None,
            remove: None,
        },
        &[],
    );
}

#[test]
#[should_panic(expected = "unauthorized hook caller")]
fn test_unauthorized_stake_changed_hook_caller() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let delegation_addr = suite.delegation_addr.clone();

    suite.execute_smart_ok(
        "not_registered_hook_caller",
        &delegation_addr,
        &crate::msg::ExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake {
            addr: Addr::unchecked("not_registered_hook_caller"),
            amount: Uint128::one(),
        }),
        &[],
    );
}

#[test]
#[should_panic(expected = "unauthorized hook caller")]
fn test_unauthorized_membership_changed_hook_caller() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let delegation_addr = suite.delegation_addr.clone();

    suite.execute_smart_ok(
        "not_registered_hook_caller",
        &delegation_addr,
        &crate::msg::ExecuteMsg::MemberChangedHook(cw4::MemberChangedHookMsg { diffs: vec![] }),
        &[],
    );
}

#[test]
#[should_panic(expected = "unauthorized hook caller")]
fn test_cw721_stake_changed_hook_authorizes() {
    let mut suite = Cw721DaoVoteDelegationTestingSuite::new().build();
    let dao = suite.dao.clone();
    let delegation_addr = suite.delegation_addr.clone();

    suite.execute_smart_ok(
        ADDR1,
        delegation_addr,
        &crate::msg::ExecuteMsg::NftStakeChangeHook(NftStakeChangedHookMsg::Stake {
            addr: Addr::unchecked(ADDR3),
            token_id: dao.x.cw721_addr.to_string(),
        }),
        &[],
    );
}

#[test]
#[should_panic(expected = "unauthorized hook caller")]
fn test_unauthorized_execute_vote_hook_caller() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let delegation_addr = suite.delegation_addr.clone();

    suite.execute_smart_ok(
        "not_registered_hook_caller",
        &delegation_addr,
        &crate::msg::ExecuteMsg::VoteHook(VoteHookMsg::NewVote {
            proposal_id: 1,
            voter: "voter".to_string(),
            vote: "vote".to_string(),
            power: Uint128::one(),
            individual_power: Uint128::one(),
            height: 1,
            is_first_vote: false,
        }),
        &[],
    );
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_unauthorized_config_update() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let delegation_addr = suite.delegation_addr.clone();

    suite.execute_smart_ok(
        "no_one",
        &delegation_addr,
        &crate::msg::ExecuteMsg::UpdateConfig {
            vp_cap_percent: OptionalUpdate(None),
            delegation_validity_blocks: OptionalUpdate(None),
            max_delegations: None,
        },
        &[],
    );
}

#[test]
fn test_migration_incorrect_contract() {
    let mut deps = mock_dependencies();

    cw2::set_contract_version(&mut deps.storage, "different_contract", "0.1.0").unwrap();

    let err =
        crate::contract::migrate(deps.as_mut(), mock_env(), crate::msg::MigrateMsg {}).unwrap_err();
    assert_eq!(
        err,
        crate::ContractError::MigrationErrorIncorrectContract {
            expected: "crates.io:dao-vote-delegation".to_string(),
            actual: "different_contract".to_string(),
        }
    );
}

#[test]
fn test_cannot_migrate_to_same_version() {
    let mut deps = mock_dependencies();

    cw2::set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();

    let err =
        crate::contract::migrate(deps.as_mut(), mock_env(), crate::msg::MigrateMsg {}).unwrap_err();
    assert_eq!(
        err,
        crate::ContractError::MigrationErrorInvalidVersion {
            new: CONTRACT_VERSION.to_string(),
            current: CONTRACT_VERSION.to_string()
        }
    );
}

#[test]
fn test_migrate() {
    let mut deps = mock_dependencies();

    cw2::set_contract_version(&mut deps.storage, CONTRACT_NAME, "2.4.0").unwrap();

    crate::contract::migrate(deps.as_mut(), mock_env(), crate::msg::MigrateMsg {}).unwrap();

    let version = cw2::get_contract_version(&deps.storage).unwrap();

    assert_eq!(version.contract, CONTRACT_NAME);
    assert_eq!(version.version, CONTRACT_VERSION);
}

#[test]
fn test_change_member_vp() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));
    suite.advance_block();

    // ensure all of ADDR0's voting power is delegated to ADDR1
    let initial_vp = suite.members[1].weight;
    suite.assert_delegate_total_delegated_vp(ADDR0, initial_vp);

    // double ADDR0's voting power
    let new_vp = initial_vp * 2;
    let dao_core_addr = suite.dao.core_addr.clone();
    let group_addr = suite.dao.x.group_addr.clone();
    suite.execute_smart_ok(
        &dao_core_addr,
        &group_addr,
        &cw4_group::msg::ExecuteMsg::UpdateMembers {
            add: vec![cw4::Member {
                addr: ADDR1.to_string(),
                weight: new_vp,
            }],
            remove: vec![],
        },
        &[],
    );
    suite.advance_block();

    // ensure all of ADDR0's new voting power is now delegated to ADDR1
    suite.assert_delegate_total_delegated_vp(ADDR0, new_vp);
}

#[test]
fn test_auto_unregister() {
    let mut suite = TokenDaoVoteDelegationTestingSuite::new().build();

    suite.register(ADDR0);

    suite.advance_block();

    suite.assert_delegate_registered(ADDR0, None);

    // unstake all tokens, which should automatically unregister the delegate
    suite.unstake(ADDR0, suite.members[0].amount);

    suite.advance_block();

    suite.assert_delegate_not_registered(ADDR0, None);
}

/// this test does not actually test gas limits, since cw-multi-test does not
/// run a real chain, but it is demonstrative of what behaviors may lead to high
/// gas usage. this test is replicated in the DAO DAO UI codebase using an
/// actual chain with gas limits.
#[test]
fn test_vp_cap_update_token_dao() {
    let mut suite = TokenDaoVoteDelegationTestingSuite::new()
        .with_vp_cap_percent(Decimal::percent(50))
        .with_delegation_validity_blocks(10)
        .with_max_delegations(100)
        .build();
    let dao = suite.dao.clone();

    // register ADDR0 as a delegate
    suite.register(ADDR0);

    // delegate 100% of every other member's voting power to ADDR0
    for member in suite.members.clone() {
        if member.address != ADDR0 {
            suite.delegate(member.address, ADDR0, Decimal::percent(100));
        }
    }

    // delegations take effect on the next block
    suite.advance_block();

    let total_vp_except_addr0 = suite
        .members
        .iter()
        .map(|m| {
            if m.address == ADDR0 {
                0
            } else {
                m.amount.into()
            }
        })
        .sum::<u128>();
    suite.assert_delegate_total_delegated_vp(ADDR0, total_vp_except_addr0);

    // propose a proposal
    let (proposal_module, id1, p1) =
        suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // ensure delegation is correctly applied to proposal and that VP cap is
    // applied correctly. effective should be 50% of total voting power, and
    // total should be everything that's delegated to ADDR0
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        // VP cap is set to 50% of total voting power
        suite
            .members
            .iter()
            .map(|m| m.amount)
            .sum::<Uint128>()
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        total_vp_except_addr0,
    );

    // change VP cap to 30% of total
    suite.update_vp_cap_percent(Some(Decimal::percent(30)));
    // updates take effect on the next block
    suite.advance_block();

    // propose another proposal
    let (_, id2, p2) = suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // ensure delegation is correctly applied to proposal and that VP cap is
    // applied correctly. effective should be 30% of total voting power, and
    // total should still be everything that's delegated to ADDR0
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        // VP cap is set to 30% of total voting power
        suite
            .members
            .iter()
            .map(|m| m.amount)
            .sum::<Uint128>()
            .mul_floor(Decimal::percent(30)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        total_vp_except_addr0,
    );

    // old proposal should still use old VP cap
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        // VP cap is set to 50% of total voting power
        suite
            .members
            .iter()
            .map(|m| m.amount)
            .sum::<Uint128>()
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        total_vp_except_addr0,
    );

    // remove VP cap
    suite.update_vp_cap_percent(None);
    // updates take effect on the next block
    suite.advance_block();

    // propose another proposal
    let (_, id3, p3) = suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // effective should now be equal to total since there is no cap
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id3,
        p3.start_height,
        total_vp_except_addr0,
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id3,
        p3.start_height,
        total_vp_except_addr0,
    );

    // old proposals should still use old VP caps
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        // VP cap is set to 30% of total voting power
        suite
            .members
            .iter()
            .map(|m| m.amount)
            .sum::<Uint128>()
            .mul_floor(Decimal::percent(30)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id2,
        p2.start_height,
        total_vp_except_addr0,
    );
    suite.assert_effective_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        // VP cap is set to 50% of total voting power
        suite
            .members
            .iter()
            .map(|m| m.amount)
            .sum::<Uint128>()
            .mul_floor(Decimal::percent(50)),
    );
    suite.assert_total_udvp(
        ADDR0,
        &proposal_module,
        id1,
        p1.start_height,
        total_vp_except_addr0,
    );
}

#[test]
fn test_gas_limits() {
    let mut suite = TokenDaoVoteDelegationTestingSuite::new()
        .with_max_delegations(100)
        .build();
    let dao = suite.dao.clone();

    // unstake all tokens for initial members
    for member in suite.members.clone() {
        suite.unstake(member.address, member.amount);
    }

    // mint 2,000 tokens and stake half for each of 1,000 members
    let members = 1_000u128;
    let initial_balance = 2_000u128;
    let initial_staked = initial_balance / 2;
    for i in 0..members {
        suite.mint(format!("member_{}", i), initial_balance);
        suite.stake(format!("member_{}", i), initial_staked);
    }

    // staking takes effect at the next block
    suite.advance_block();

    let total_vp: dao_interface::voting::TotalPowerAtHeightResponse = suite
        .querier()
        .query_wasm_smart(
            &suite.dao.core_addr,
            &dao_voting_token_staked::msg::QueryMsg::TotalPowerAtHeight { height: None },
        )
        .unwrap();
    assert_eq!(total_vp.power, Uint128::from(initial_staked * members));

    // register first 100 members as delegates, and make delegator the first
    // non-delegate
    let delegates = 100u128;
    let delegator = format!("member_{}", delegates);
    for i in 0..delegates {
        suite.register(format!("member_{}", i));
    }

    // delegations take effect on the next block
    suite.advance_block();

    // check that the delegations are registered
    suite.assert_delegates_count(100);

    // TEST 1: Update voting power for a delegator, which loops through all
    // delegates and updates their delegated voting power. This should cause a
    // gas error if there are too many delegates to update.

    // delegate to each of the delegates, rounding to 5 decimal places to avoid
    // infinitely repeating decimals
    let percent_delegated = Decimal::from_ratio(100_000u128 / delegates / 3, 100_000u128);
    for i in 0..delegates {
        suite.delegate(&delegator, format!("member_{}", i), percent_delegated);
    }

    // delegations take effect on the next block
    suite.advance_block();

    // check that the voting power is distributed correctly
    for delegate in suite.delegates(None, None) {
        assert_eq!(
            delegate.power,
            Uint128::from(initial_staked).mul_floor(percent_delegated)
        );
    }

    // stake the other half of the tokens for the delegator, which should loop
    // through and update all delegations
    suite.stake(&delegator, initial_balance - initial_staked);

    // delegations take effect on the next block
    suite.advance_block();

    // check that the voting power is distributed correctly
    for delegate in suite.delegates(None, None) {
        assert_eq!(
            delegate.power,
            Uint128::from(initial_balance).mul_floor(percent_delegated)
        );
    }

    // undo the half stake so that all members have the same voting power again
    suite.unstake(&delegator, initial_balance - initial_staked);

    // delegations take effect on the next block
    suite.advance_block();

    // TEST 2: Override all delegates' votes, which loops through all delegates
    // and updates both their ballots and unvoted delegated voting power on that
    // proposal. This should cause a gas error if there are too many delegates
    // to update.

    let (proposal_module, proposal_id, proposal) =
        suite.propose_single_choice(&dao, "member_0", "test proposal", vec![]);

    // ensure that the unvoted delegated voting power is equal to the total
    // delegated voting power, since the delegator has not voted yet
    for i in 0..delegates {
        let vp = Uint128::from(initial_staked).mul_floor(percent_delegated);
        suite.assert_effective_udvp(
            format!("member_{}", i),
            &proposal_module,
            proposal_id,
            proposal.start_height,
            vp,
        );
        suite.assert_total_udvp(
            format!("member_{}", i),
            &proposal_module,
            proposal_id,
            proposal.start_height,
            vp,
        );
    }

    // all delegates vote on the proposal
    for i in 0..delegates {
        suite.vote_single_choice(
            &dao,
            format!("member_{}", i),
            proposal_id,
            dao_voting::voting::Vote::Yes,
        );
    }

    // verify votes tallied with the delegates' personal voting power and
    // delegated voting power
    suite.assert_single_choice_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::Yes,
        // compute delegated voting power
        Uint128::from(initial_staked)
            .mul_floor(percent_delegated)
            // add personal voting power
            .strict_add(Uint128::from(initial_staked))
            // multiply by number of delegates
            .checked_mul(Uint128::from(delegates))
            .unwrap(),
    );

    // delegator overrides all delegates' votes, which should update all
    // delegate's ballots and unvoted delegated voting power on the proposal
    suite.vote_single_choice(&dao, delegator, proposal_id, dao_voting::voting::Vote::No);

    // verify vote tallies have been updated with the delegator's vote, removing
    // the delegator's delegated voting power from the delegates' yes votes and
    // adding the delegator's full voting power to the no votes
    suite.assert_single_choice_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::Yes,
        // add personal voting power
        Uint128::from(initial_staked)
            // multiply by number of delegates
            .checked_mul(Uint128::from(delegates))
            .unwrap(),
    );
    suite.assert_single_choice_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::No,
        Uint128::from(initial_staked),
    );

    // verify that the unvoted delegated voting power is 0, since the delegator
    // voted
    for i in 0..delegates {
        suite.assert_effective_udvp(
            format!("member_{}", i),
            &proposal_module,
            proposal_id,
            proposal.start_height,
            0u128,
        );
        suite.assert_total_udvp(
            format!("member_{}", i),
            &proposal_module,
            proposal_id,
            proposal.start_height,
            0u128,
        );
    }
}

#[test]
fn test_revote() {
    let mut suite = Cw4DaoVoteDelegationTestingSuite::new().build();
    let dao = suite.dao.clone();

    // Enable revoting on both proposal modules.
    suite
        .execute_smart(
            &dao.core_addr,
            &dao.proposal_modules[0].1,
            &dao_proposal_single::msg::ExecuteMsg::UpdateConfig {
                threshold: dao_voting::threshold::Threshold::AbsolutePercentage {
                    percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                },
                max_voting_period: Duration::Height(10),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: true,
                dao: dao.core_addr.to_string(),
                close_proposal_on_execution_failure: true,
                veto: None,
            },
            &[],
        )
        .unwrap();
    suite
        .execute_smart(
            &dao.core_addr,
            &dao.proposal_modules[1].1,
            &dao_proposal_multiple::msg::ExecuteMsg::UpdateConfig {
                voting_strategy: dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                    quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                },
                max_voting_period: Duration::Height(10),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: true,
                dao: dao.core_addr.to_string(),
                close_proposal_on_execution_failure: true,
                veto: None,
            },
            &[],
        )
        .unwrap();

    // register member as delegate
    suite.register(ADDR0);

    // delegate 100% of voting power to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));

    // delegations take effect on the next block
    suite.advance_block();

    // SINGLE CHOICE PROPOSAL

    // propose a single choice proposal
    let (proposal_module, proposal_id, _) =
        suite.propose_single_choice(&dao, ADDR0, "test proposal", vec![]);

    // delegate votes on the proposal
    suite.vote_single_choice(&dao, ADDR0, proposal_id, dao_voting::voting::Vote::Yes);

    // assert vote count on single choice proposal
    suite.assert_single_choice_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::Yes,
        // both members' weight is used by delegate
        suite.members[0].weight + suite.members[1].weight,
    );
    // assert individual vote count on single choice proposal
    suite.assert_single_choice_individual_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::Yes,
        // only delegate weight is counted in individual votes
        suite.members[0].weight,
    );

    // revote on the proposal
    suite.vote_single_choice(&dao, ADDR0, proposal_id, dao_voting::voting::Vote::No);

    // assert vote count on single choice proposal
    suite.assert_single_choice_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::Yes,
        0u128,
    );
    suite.assert_single_choice_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::No,
        // both members' weight is used by delegate
        suite.members[0].weight + suite.members[1].weight,
    );
    // assert individual vote count on single choice proposal
    suite.assert_single_choice_individual_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::Yes,
        0u128,
    );
    suite.assert_single_choice_individual_votes_count(
        &proposal_module,
        proposal_id,
        dao_voting::voting::Vote::No,
        // only delegate weight is counted in individual votes
        suite.members[0].weight,
    );

    // MULTIPLE CHOICE PROPOSAL

    // propose a multiple choice proposal
    let (proposal_module, proposal_id, _) = suite.propose_multiple_choice(
        &dao,
        ADDR0,
        "test proposal",
        vec![
            dao_voting::multiple_choice::MultipleChoiceOption {
                title: "Option 1".to_string(),
                description: "Option 1 description".to_string(),
                msgs: vec![],
            },
            dao_voting::multiple_choice::MultipleChoiceOption {
                title: "Option 2".to_string(),
                description: "Option 2 description".to_string(),
                msgs: vec![],
            },
            dao_voting::multiple_choice::MultipleChoiceOption {
                title: "Option 3".to_string(),
                description: "Option 3 description".to_string(),
                msgs: vec![],
            },
        ],
    );

    // delegate votes on the proposal
    suite.vote_multiple_choice(&dao, ADDR0, proposal_id, 1);

    // assert vote count on multiple choice proposal
    suite.assert_multiple_choice_votes_count(
        &proposal_module,
        proposal_id,
        1,
        // both members' weight is used by delegate
        suite.members[0].weight + suite.members[1].weight,
    );
    // assert individual vote count on multiple choice proposal
    suite.assert_multiple_choice_individual_votes_count(
        &proposal_module,
        proposal_id,
        1,
        // only delegate weight is counted in individual votes
        suite.members[0].weight,
    );

    // revote on the proposal
    suite.vote_multiple_choice(&dao, ADDR0, proposal_id, 2);

    // assert vote count on multiple choice proposal
    suite.assert_multiple_choice_votes_count(&proposal_module, proposal_id, 1, 0u128);
    suite.assert_multiple_choice_votes_count(
        &proposal_module,
        proposal_id,
        2,
        // both members' weight is used by delegate
        suite.members[0].weight + suite.members[1].weight,
    );
    // assert individual vote count on multiple choice proposal
    suite.assert_multiple_choice_individual_votes_count(&proposal_module, proposal_id, 1, 0u128);
    suite.assert_multiple_choice_individual_votes_count(
        &proposal_module,
        proposal_id,
        2,
        // only delegate weight is counted in individual votes
        suite.members[0].weight,
    );
}
