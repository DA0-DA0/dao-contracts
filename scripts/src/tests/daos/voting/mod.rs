use crate::tests::{gauges::suite::DaoDaoCw4Gauge, Cw4VotingInitMsg};
use cosmwasm_std::{to_json_binary, Uint128};
use cw4::Member;
use cw_orch::{anyhow, prelude::*};
use dao_interface::state::ModuleInstantiateInfo;
use dao_voting::{pre_propose::PreProposeInfo, threshold::Threshold};
use dao_voting_cw4::msg::GroupContract;

use super::extract_dao_events;

pub fn dao_cw4_voting_template(
    mock: MockBech32,
    suite: &mut DaoDaoCw4Gauge<MockBech32>,
    initial_members: Vec<Member>,
) -> anyhow::Result<Vec<Addr>> {
    // setup cw4 stuff

    let res = suite.dao_core.instantiate(
        &dao_interface::msg::InstantiateMsg {
            admin: None,
            name: "Cw4VotingDao".into(),
            description: "template for dao with cw4 voting module".into(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: suite.cw4_vote.code_id().unwrap(),
                msg: to_json_binary(&Cw4VotingInitMsg {
                    group_contract: GroupContract::New {
                        cw4_group_code_id: suite.cw4_group.unwrap(),
                        initial_members,
                    },
                })?,
                admin: None,
                funds: vec![],
                label: "cw4-voting".into(),
            },
            proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                code_id: suite.prop_single.code_id()?,
                msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                    threshold: Threshold::AbsoluteCount {
                        threshold: Uint128::one(),
                    },
                    max_voting_period: cw_utils::Duration::Height(mock.block_info()?.height + 10),
                    min_voting_period: None,
                    only_members_execute: true,
                    allow_revoting: false,
                    pre_propose_info: PreProposeInfo::AnyoneMayPropose {},
                    close_proposal_on_execution_failure: true,
                    veto: None,
                })?,
                admin: None,
                funds: vec![],
                label: "single-proposal".into(),
            }],
            initial_items: None,
            dao_uri: None,
        },
        None,
        None,
    )?;

    // grabs the daos created prop modules.
    // Here we only expect one contract per module but more complex daos may have more than one, will need to update.
    let prop_addr = extract_dao_events(&res.events, "prop_module").unwrap();
    let voting_addr = extract_dao_events(&res.events, "voting_module").unwrap();

    Ok(vec![prop_addr, voting_addr])
}
