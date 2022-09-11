use cosmwasm_std::{Addr, Uint128};
use cw_core::state::{ProposalModule, ProposalModuleStatus};
use cw_multi_test::App;

use cw_pre_propose_base_proposal_multiple as cppbpm;
use indexable_hooks::HooksResponse;
use voting::pre_propose::ProposalCreationPolicy;

use crate::{
    msg::QueryMsg,
    query::{ProposalListResponse, ProposalResponse},
    state::Config,
};

pub(crate) fn query_deposit_config_and_pre_propose_module(
    app: &App,
    proposal_multiple: &Addr,
) -> (cppbpm::Config, Addr) {
    let config = query_proposal_config(app, proposal_multiple);

    if let ProposalCreationPolicy::Module { addr: module_addr } = config.proposal_creation_policy {
        let deposit_config = query_pre_proposal_multiple_config(app, &module_addr);

        (deposit_config, module_addr)
    } else {
        panic!("no pre-propose module.")
    }
}

pub(crate) fn query_proposal_config(app: &App, proposal_multiple: &Addr) -> Config {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::Config {})
        .unwrap()
}

pub(crate) fn query_pre_proposal_multiple_config(app: &App, pre_propose: &Addr) -> cppbpm::Config {
    app.wrap()
        .query_wasm_smart(pre_propose, &cppbpm::QueryMsg::Config {})
        .unwrap()
}

pub(crate) fn query_pre_proposal_multiple_deposit_info(
    app: &App,
    pre_propose: &Addr,
    proposal_id: u64,
) -> cppbpm::DepositInfoResponse {
    app.wrap()
        .query_wasm_smart(pre_propose, &cppbpm::QueryMsg::DepositInfo { proposal_id })
        .unwrap()
}

pub(crate) fn query_multiple_proposal_module(app: &App, core_addr: &Addr) -> Addr {
    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr,
            &cw_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // Filter out disabled modules.
    let modules = modules
        .into_iter()
        .filter(|module| module.status == ProposalModuleStatus::Enabled)
        .collect::<Vec<_>>();

    assert_eq!(
        modules.len(),
        1,
        "wrong proposal module count. expected 1, got {}",
        modules.len()
    );

    modules.into_iter().next().unwrap().address
}

pub(crate) fn query_list_proposals(
    app: &App,
    proposal_multiple: &Addr,
    start_after: Option<u64>,
    limit: Option<u64>,
) -> ProposalListResponse {
    app.wrap()
        .query_wasm_smart(
            proposal_multiple,
            &QueryMsg::ListProposals { start_after, limit },
        )
        .unwrap()
}

pub(crate) fn query_proposal_hooks(app: &App, proposal_multiple: &Addr) -> HooksResponse {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::ProposalHooks {})
        .unwrap()
}

pub(crate) fn query_vote_hooks(app: &App, proposal_multiple: &Addr) -> HooksResponse {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::VoteHooks {})
        .unwrap()
}

pub(crate) fn query_list_proposals_reverse(
    app: &App,
    proposal_multiple: &Addr,
    start_before: Option<u64>,
    limit: Option<u64>,
) -> ProposalListResponse {
    app.wrap()
        .query_wasm_smart(
            proposal_multiple,
            &QueryMsg::ReverseProposals {
                start_before,
                limit,
            },
        )
        .unwrap()
}

pub(crate) fn query_dao_token(app: &App, core_addr: &Addr) -> Addr {
    let voting_module = query_voting_module(app, core_addr);
    app.wrap()
        .query_wasm_smart(
            voting_module,
            &cw_core_interface::voting::Query::TokenContract {},
        )
        .unwrap()
}

pub(crate) fn query_voting_module(app: &App, core_addr: &Addr) -> Addr {
    app.wrap()
        .query_wasm_smart(core_addr, &cw_core::msg::QueryMsg::VotingModule {})
        .unwrap()
}

pub(crate) fn query_balance_cw20<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = cw20::Cw20QueryMsg::Balance {
        address: address.into(),
    };
    let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

pub(crate) fn query_balance_native(app: &App, who: &str, denom: &str) -> Uint128 {
    let res = app.wrap().query_balance(who, denom).unwrap();
    res.amount
}

pub(crate) fn query_proposal(app: &App, proposal_multiple: &Addr, id: u64) -> ProposalResponse {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::Proposal { proposal_id: id })
        .unwrap()
}
