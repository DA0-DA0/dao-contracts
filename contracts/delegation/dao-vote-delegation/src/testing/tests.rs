use cosmwasm_std::{Addr, Decimal, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};
use dao_testing::{DaoTestingSuite, DaoTestingSuiteBase, Executor, ADDR0, ADDR1, ADDR2};
use dao_voting::delegation::{DelegationResponse, DelegationsResponse};

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
fn test_setup() {
    let mut base = DaoTestingSuiteBase::base();
    let mut suite = base.cw4();
    let dao = suite.dao();

    let code_id = suite.base.app.store_code(dao_vote_delegation_contract());
    let delegation_addr = suite
        .base
        .app
        .instantiate_contract(
            code_id,
            dao.core_addr.clone(),
            &crate::msg::InstantiateMsg {
                dao: None,
                vp_hook_callers: Some(vec![dao.x.group_addr.to_string()]),
                no_sync_proposal_modules: None,
                vp_cap_percent: Some(Decimal::percent(50)),
                delegation_validity_blocks: Some(100),
            },
            &[],
            "delegation",
            None,
        )
        .unwrap();

    // register addr0 as a delegate
    suite
        .base
        .app
        .execute_contract(
            Addr::unchecked(ADDR0),
            delegation_addr.clone(),
            &crate::msg::ExecuteMsg::Register {},
            &[],
        )
        .unwrap();

    // delegate 100% of addr1's voting power to addr0
    suite
        .base
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            delegation_addr.clone(),
            &crate::msg::ExecuteMsg::Delegate {
                delegate: ADDR0.to_string(),
                percent: Decimal::percent(100),
            },
            &[],
        )
        .unwrap();

    // delegations take effect on the next block
    suite.base.advance_block();

    let delegations: DelegationsResponse = suite
        .querier()
        .query_wasm_smart(
            &delegation_addr,
            &crate::msg::QueryMsg::Delegations {
                delegator: ADDR1.to_string(),
                height: None,
                offset: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(delegations.delegations.len(), 1);
    assert_eq!(
        delegations.delegations[0],
        DelegationResponse {
            delegate: Addr::unchecked(ADDR0),
            percent: Decimal::percent(100),
            active: true,
        }
    );

    // propose a proposal
    let (proposal_module, id1, p1) =
        dao.propose_single_choice(&mut suite.base.app, ADDR0, "test proposal 1", vec![]);

    // ensure delegation is correctly applied to proposal
    let udvp: dao_voting::delegation::UnvotedDelegatedVotingPowerResponse = suite
        .querier()
        .query_wasm_smart(
            &delegation_addr,
            &crate::msg::QueryMsg::UnvotedDelegatedVotingPower {
                delegate: ADDR0.to_string(),
                proposal_module: proposal_module.to_string(),
                proposal_id: id1,
                height: p1.start_height,
            },
        )
        .unwrap();
    assert_eq!(
        udvp.effective,
        Uint128::from(suite.members[1].weight as u128)
    );

    // set delegation to 50%
    suite
        .base
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            delegation_addr.clone(),
            &crate::msg::ExecuteMsg::Delegate {
                delegate: ADDR0.to_string(),
                percent: Decimal::percent(50),
            },
            &[],
        )
        .unwrap();

    // delegations take effect on the next block
    suite.base.advance_block();

    // propose a proposal
    let (_, id2, p2) =
        dao.propose_single_choice(&mut suite.base.app, ADDR2, "test proposal 2", vec![]);

    // ensure delegation is correctly applied to new proposal
    let udvp: dao_voting::delegation::UnvotedDelegatedVotingPowerResponse = suite
        .querier()
        .query_wasm_smart(
            &delegation_addr,
            &crate::msg::QueryMsg::UnvotedDelegatedVotingPower {
                delegate: ADDR0.to_string(),
                proposal_module: proposal_module.to_string(),
                proposal_id: id2,
                height: p2.start_height,
            },
        )
        .unwrap();
    assert_eq!(
        udvp.effective,
        Uint128::from((suite.members[1].weight / 2) as u128)
    );

    // ensure old delegation is still applied to old proposal
    let udvp: dao_voting::delegation::UnvotedDelegatedVotingPowerResponse = suite
        .querier()
        .query_wasm_smart(
            &delegation_addr,
            &crate::msg::QueryMsg::UnvotedDelegatedVotingPower {
                delegate: ADDR0.to_string(),
                proposal_module: proposal_module.to_string(),
                proposal_id: id1,
                height: p1.start_height,
            },
        )
        .unwrap();
    assert_eq!(
        udvp.effective,
        Uint128::from(suite.members[1].weight as u128)
    );
}
