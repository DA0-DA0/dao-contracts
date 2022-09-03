use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Attribute, Reply, SubMsgResult, Uint128,
};
use voting::{
    reply::{mask_proposal_execution_proposal_id, mask_proposal_hook_index, mask_vote_hook_index},
    status::Status,
    threshold::{PercentageThreshold, Threshold},
    voting::Votes,
};

use crate::{
    contract::reply,
    proposal::SingleChoiceProposal,
    state::{PROPOSALS, PROPOSAL_HOOKS, VOTE_HOOKS},
};

const CREATOR_ADDR: &str = "creator";

#[test]
fn test_reply_proposal_mock() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let m_proposal_id = mask_proposal_execution_proposal_id(1);
    PROPOSALS
        .save(
            deps.as_mut().storage,
            1,
            &SingleChoiceProposal {
                title: "A simple text proposal".to_string(),
                description: "This is a simple text proposal".to_string(),
                proposer: Addr::unchecked(CREATOR_ADDR),
                start_height: env.block.height,
                expiration: cw_utils::Duration::Height(6).after(&env.block),
                min_voting_period: None,
                threshold: Threshold::AbsolutePercentage {
                    percentage: PercentageThreshold::Majority {},
                },
                allow_revoting: false,
                total_power: Uint128::new(100_000_000),
                msgs: vec![],
                status: Status::Open,
                votes: Votes::zero(),
                created: env.block.time,
                last_updated: env.block.time,
            },
        )
        .unwrap();

    // PROPOSALS
    let reply_msg = Reply {
        id: m_proposal_id,
        result: SubMsgResult::Err("error_msg".to_string()),
    };
    let res = reply(deps.as_mut(), env, reply_msg).unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute {
            key: "proposal execution failed".to_string(),
            value: 1.to_string()
        }
    );

    let prop = PROPOSALS.load(deps.as_mut().storage, 1).unwrap();
    assert_eq!(prop.status, Status::ExecutionFailed);
}

#[test]
fn test_reply_hooks_mock() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    // Proposal hook
    let m_proposal_hook_idx = mask_proposal_hook_index(0);
    PROPOSAL_HOOKS
        .add_hook(deps.as_mut().storage, Addr::unchecked(CREATOR_ADDR))
        .unwrap();

    let reply_msg = Reply {
        id: m_proposal_hook_idx,
        result: SubMsgResult::Err("error_msg".to_string()),
    };
    let res = reply(deps.as_mut(), env.clone(), reply_msg).unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute {
            key: "removed proposal hook".to_string(),
            value: format! {"{CREATOR_ADDR}:{}", 0}
        }
    );

    // Vote hook
    let m_vote_hook_idx = mask_vote_hook_index(0);
    VOTE_HOOKS
        .add_hook(deps.as_mut().storage, Addr::unchecked(CREATOR_ADDR))
        .unwrap();

    let reply_msg = Reply {
        id: m_vote_hook_idx,
        result: SubMsgResult::Err("error_msg".to_string()),
    };
    let res = reply(deps.as_mut(), env, reply_msg).unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute {
            key: "removed vote hook".to_string(),
            value: format! {"{CREATOR_ADDR}:{}", 0}
        }
    );
}
