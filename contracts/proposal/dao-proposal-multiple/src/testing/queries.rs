use cosmwasm_std::{Addr, Uint128};
use cw_hooks::HooksResponse;
use cw_multi_test::App;
use dao_interface::state::{ProposalModule, ProposalModuleStatus};
use dao_pre_propose_multiple as cppm;
use dao_voting::pre_propose::ProposalCreationPolicy;

use crate::{
    msg::QueryMsg,
    query::{ProposalListResponse, ProposalResponse},
    state::Config,
};

pub fn query_deposit_config_and_pre_propose_module(
    app: &App,
    proposal_multiple: &Addr,
) -> (cppm::Config, Addr) {
    let proposal_creation_policy = query_creation_policy(app, proposal_multiple);

    if let ProposalCreationPolicy::Module { addr: module_addr } = proposal_creation_policy {
        let deposit_config = query_pre_proposal_multiple_config(app, &module_addr);

        (deposit_config, module_addr)
    } else {
        panic!("no pre-propose module.")
    }
}

pub fn query_proposal_config(app: &App, proposal_multiple: &Addr) -> Config {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::Config {})
        .unwrap()
}

pub fn query_creation_policy(app: &App, proposal_multiple: &Addr) -> ProposalCreationPolicy {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::ProposalCreationPolicy {})
        .unwrap()
}

pub fn query_pre_proposal_multiple_config(app: &App, pre_propose: &Addr) -> cppm::Config {
    app.wrap()
        .query_wasm_smart(pre_propose, &cppm::QueryMsg::Config {})
        .unwrap()
}

pub fn query_multiple_proposal_module(app: &App, core_addr: &Addr) -> Addr {
    let modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr,
            &dao_interface::msg::QueryMsg::ProposalModules {
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

pub fn query_list_proposals(
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

pub fn query_proposal_hooks(app: &App, proposal_multiple: &Addr) -> HooksResponse {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::ProposalHooks {})
        .unwrap()
}

pub fn query_vote_hooks(app: &App, proposal_multiple: &Addr) -> HooksResponse {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::VoteHooks {})
        .unwrap()
}

pub fn query_list_proposals_reverse(
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

pub fn query_dao_token(app: &App, core_addr: &Addr) -> Addr {
    let voting_module = query_voting_module(app, core_addr);
    app.wrap()
        .query_wasm_smart(
            voting_module,
            &dao_interface::voting::Query::TokenContract {},
        )
        .unwrap()
}

pub fn query_voting_module(app: &App, core_addr: &Addr) -> Addr {
    app.wrap()
        .query_wasm_smart(core_addr, &dao_interface::msg::QueryMsg::VotingModule {})
        .unwrap()
}

pub fn query_cw20_token_staking_contracts(app: &App, core_addr: &Addr) -> (Addr, Addr) {
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(core_addr, &dao_interface::msg::QueryMsg::VotingModule {})
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    (token_contract, staking_contract)
}

pub fn query_balance_cw20<T: Into<String>, U: Into<String>>(
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

pub fn query_balance_native(app: &App, who: &str, denom: &str) -> Uint128 {
    let res = app.wrap().query_balance(who, denom).unwrap();
    res.amount
}

pub fn query_proposal(app: &App, proposal_multiple: &Addr, id: u64) -> ProposalResponse {
    app.wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::Proposal { proposal_id: id })
        .unwrap()
}
