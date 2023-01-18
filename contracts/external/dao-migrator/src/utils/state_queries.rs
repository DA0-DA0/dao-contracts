use cosmwasm_std::{Addr, Deps, StdResult, Uint128};

use crate::types::SingleProposalData;

use super::query_helpers::{
    v1_expiration_to_v2, v1_status_to_v2, v1_threshold_to_v2, v1_votes_to_v2,
};

// pub fn query_core_dump_state_v1(
//     deps: Deps,
//     core_addr: &Addr,
// ) -> Result<TestCoreDumpState, ContractError> {
//     let dump_state: cw_core_v1::query::DumpStateResponse = deps
//         .querier
//         .query_wasm_smart(core_addr, &cw_core_v1::msg::QueryMsg::DumpState {})?;

//     let proposals = dump_state
//         .proposal_modules
//         .clone()
//         .into_iter()
//         .enumerate()
//         .map(|(idx, address)| ProposalModule {
//             address,
//             prefix: derive_proposal_module_prefix(idx).unwrap(),
//             status: ProposalModuleStatus::Enabled {},
//         })
//         .collect::<Vec<ProposalModule>>();

//     Ok(TestCoreDumpState {
//         proposal_modules: proposals,
//         voting_module: dump_state.voting_module,
//         total_proposal_module_count: dump_state.proposal_modules.len() as u32,
//     })
// }

// pub fn query_core_dump_state_v2(
//     deps: Deps,
//     core_addr: &Addr,
// ) -> Result<TestCoreDumpState, ContractError> {
//     let dump_state: dao_core::query::DumpStateResponse = deps
//         .querier
//         .query_wasm_smart(core_addr, &dao_core::msg::QueryMsg::DumpState {})?;

//     Ok(TestCoreDumpState {
//         proposal_modules: dump_state.proposal_modules.clone(),
//         voting_module: dump_state.voting_module,
//         total_proposal_module_count: dump_state.proposal_modules.len() as u32,
//     })
// }

// pub fn query_core_items_v1(
//     deps: Deps,
//     core_addr: &Addr,
// ) -> Result<Vec<(String, String)>, ContractError> {
//     let items: Vec<(String, String)> = deps.querier.query_wasm_smart(
//         core_addr,
//         &cw_core_v1::msg::QueryMsg::ListItems {
//             start_at: None,
//             limit: None,
//         },
//     )?;

//     Ok(items)
// }

// pub fn query_core_items_v2(
//     deps: Deps,
//     core_addr: &Addr,
// ) -> Result<Vec<(String, String)>, ContractError> {
//     let items: Vec<(String, String)> = deps.querier.query_wasm_smart(
//         core_addr,
//         &dao_core::msg::QueryMsg::ListItems {
//             start_after: None,
//             limit: None,
//         },
//     )?;

//     Ok(items)
// }

// TODO: we do several loops over proposal addrs, if V1 only have 1 proposal module, then its fine.
// but if we gonna have 2 or more, we better run over the vec once, and get the data from that loop.
pub fn query_proposal_count_v1(deps: Deps, proposals_addrs: Vec<String>) -> StdResult<Vec<u64>> {
    proposals_addrs
        .into_iter()
        .map(|proposal_addr| {
            deps.querier.query_wasm_smart(
                proposal_addr,
                &cw_proposal_single_v1::msg::QueryMsg::ProposalCount {},
            )
        })
        .collect()
}

pub fn query_proposal_count_v2(deps: Deps, proposals_addrs: Vec<String>) -> StdResult<Vec<u64>> {
    proposals_addrs
        .into_iter()
        .map(|proposal_addr| {
            deps.querier.query_wasm_smart(
                proposal_addr,
                &dao_proposal_single::msg::QueryMsg::ProposalCount {},
            )
        })
        .collect()
}

pub fn query_proposal_v1(
    deps: Deps,
    proposals_addrs: Vec<String>,
) -> StdResult<(
    Vec<dao_proposal_single::proposal::SingleChoiceProposal>,
    SingleProposalData,
)> {
    let mut last_proposal = None;

    let proposals = proposals_addrs
        .into_iter()
        .map(|proposal_addr| {
            println!("{:?}", proposal_addr);
            let proposals: cw_proposal_single_v1::query::ProposalListResponse = deps
                .querier
                .query_wasm_smart(
                    proposal_addr,
                    &cw_proposal_single_v1::msg::QueryMsg::ReverseProposals {
                        start_before: None,
                        limit: None,
                    },
                )
                .unwrap();

            // TODO: What happens when there is no proposals on the module? we should ignore testing it
            // and handle it as "tested?"
            let proposal = proposals.proposals.first().unwrap().proposal.clone();

            if last_proposal.is_none() {
                last_proposal = Some(SingleProposalData {
                    proposer: proposal.proposer.clone(),
                    start_height: proposal.start_height,
                });
            }

            dao_proposal_single::proposal::SingleChoiceProposal {
                title: proposal.title,
                description: proposal.description,
                proposer: proposal.proposer,
                start_height: proposal.start_height,
                min_voting_period: proposal.min_voting_period.map(v1_expiration_to_v2),
                expiration: v1_expiration_to_v2(proposal.expiration),
                threshold: v1_threshold_to_v2(proposal.threshold),
                total_power: proposal.total_power,
                msgs: proposal.msgs,
                status: v1_status_to_v2(proposal.status),
                votes: v1_votes_to_v2(proposal.votes),
                allow_revoting: proposal.allow_revoting,
            }
        })
        .collect::<Vec<dao_proposal_single::proposal::SingleChoiceProposal>>();

    Ok((proposals, last_proposal.unwrap()))
}

pub fn query_proposal_v2(
    deps: Deps,
    proposals_addrs: Vec<String>,
) -> StdResult<(
    Vec<dao_proposal_single::proposal::SingleChoiceProposal>,
    SingleProposalData,
)> {
    let mut last_proposal = None;

    let proposals = proposals_addrs
        .into_iter()
        .map(|proposal_addr| {
            let proposals: dao_proposal_single::query::ProposalListResponse = deps
                .querier
                .query_wasm_smart(
                    proposal_addr,
                    &dao_proposal_single::msg::QueryMsg::ReverseProposals {
                        start_before: None,
                        limit: None,
                    },
                )
                .unwrap();

            let proposal = proposals.proposals.first().unwrap().proposal.clone();

            if last_proposal.is_none() {
                last_proposal = Some(SingleProposalData {
                    proposer: proposal.proposer.clone(),
                    start_height: proposal.start_height,
                });
            }

            proposal
        })
        .collect::<Vec<dao_proposal_single::proposal::SingleChoiceProposal>>();

    Ok((proposals, last_proposal.unwrap()))
}

pub fn query_total_voting_power_v1(
    deps: Deps,
    voting_addr: String,
    height: u64,
) -> StdResult<Uint128> {
    let res: cw_core_interface_v1::voting::TotalPowerAtHeightResponse =
        deps.querier.query_wasm_smart(
            voting_addr,
            &cw20_staked_balance_voting_v1::msg::QueryMsg::TotalPowerAtHeight {
                height: Some(height),
            },
        )?;
    Ok(res.power)
}

pub fn query_total_voting_power_v2(
    deps: Deps,
    voting_addr: String,
    height: u64,
) -> StdResult<Uint128> {
    let res: dao_interface::voting::TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_addr,
        &dao_voting_cw20_staked::msg::QueryMsg::TotalPowerAtHeight {
            height: Some(height),
        },
    )?;
    Ok(res.power)
}

pub fn query_single_voting_power_v1(
    deps: Deps,
    voting_addr: String,
    address: Addr,
    height: u64,
) -> StdResult<Uint128> {
    let res: cw_core_interface_v1::voting::VotingPowerAtHeightResponse =
        deps.querier.query_wasm_smart(
            voting_addr,
            &cw20_staked_balance_voting_v1::msg::QueryMsg::VotingPowerAtHeight {
                address: address.into(),
                height: Some(height),
            },
        )?;
    Ok(res.power)
}

pub fn query_single_voting_power_v2(
    deps: Deps,
    voting_addr: String,
    address: Addr,
    height: u64,
) -> StdResult<Uint128> {
    let res: dao_interface::voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_addr,
        &dao_voting_cw20_staked::msg::QueryMsg::VotingPowerAtHeight {
            address: address.into(),
            height: Some(height),
        },
    )?;
    Ok(res.power)
}
