use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};
use dao_testing::{ADDR0, ADDR1, ADDR2, ADDR3, ADDR4};

use super::*;

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
    let mut suite = DaoVoteDelegationTestingSuite::new()
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

    // register ADDR0 as a delegate
    suite.register(ADDR0);
    suite.assert_delegates_count(1);
    suite.assert_registered(ADDR0);

    // delegate 100% of addr1's voting power to ADDR0
    suite.delegate(ADDR1, ADDR0, Decimal::percent(100));

    // delegations take effect on the next block
    suite.advance_block();

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

    // unregister ADDR0 as a delegate
    suite.unregister(ADDR0);

    // delegations take effect on the next block
    suite.advance_block();

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
fn test_vp_cap_update() {
    let mut suite = DaoVoteDelegationTestingSuite::new()
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
    let mut suite = DaoVoteDelegationTestingSuite::new()
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
fn test_update_hook_callers() {
    let mut suite = DaoVoteDelegationTestingSuite::new().build();
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
                funds: vec![],
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
    let mut suite = DaoVoteDelegationTestingSuite::new().build();
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
