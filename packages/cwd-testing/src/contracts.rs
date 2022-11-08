use cosmwasm_std::Empty;

use cw_multi_test::{Contract, ContractWrapper};
use cwd_pre_propose_multiple as cppm;
use cwd_pre_propose_single as cpps;

pub fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn cw4_group_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw4_group::contract::execute,
        cw4_group::contract::instantiate,
        cw4_group::contract::query,
    );
    Box::new(contract)
}

pub fn cw721_base_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub fn cw20_stake_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
    );
    Box::new(contract)
}

pub fn v1_proposal_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_proposal_single_v1::contract::execute,
        cw_proposal_single_v1::contract::instantiate,
        cw_proposal_single_v1::contract::query,
    )
    .with_reply(cw_proposal_single_v1::contract::reply)
    .with_migrate(cw_proposal_single_v1::contract::migrate);
    Box::new(contract)
}

pub fn proposal_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_proposal_single::contract::execute,
        cwd_proposal_single::contract::instantiate,
        cwd_proposal_single::contract::query,
    )
    .with_reply(cwd_proposal_single::contract::reply)
    .with_migrate(cwd_proposal_single::contract::migrate);
    Box::new(contract)
}

pub fn pre_propose_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cpps::contract::execute,
        cpps::contract::instantiate,
        cpps::contract::query,
    );
    Box::new(contract)
}

pub fn pre_propose_multiple_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cppm::contract::execute,
        cppm::contract::instantiate,
        cppm::contract::query,
    );
    Box::new(contract)
}

pub fn cw20_staked_balances_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_voting_cw20_staked::contract::execute,
        cwd_voting_cw20_staked::contract::instantiate,
        cwd_voting_cw20_staked::contract::query,
    )
    .with_reply(cwd_voting_cw20_staked::contract::reply);
    Box::new(contract)
}

pub fn cw20_balances_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_voting_cw20_balance::contract::execute,
        cwd_voting_cw20_balance::contract::instantiate,
        cwd_voting_cw20_balance::contract::query,
    )
    .with_reply(cwd_voting_cw20_balance::contract::reply);
    Box::new(contract)
}

pub fn native_staked_balances_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_voting_native_staked::contract::execute,
        cwd_voting_native_staked::contract::instantiate,
        cwd_voting_native_staked::contract::query,
    );
    Box::new(contract)
}

pub fn voting_cw721_staked_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_voting_cw721_staked::contract::execute,
        cwd_voting_cw721_staked::contract::instantiate,
        cwd_voting_cw721_staked::contract::query,
    );
    Box::new(contract)
}

pub fn cwd_core_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_core::contract::execute,
        cwd_core::contract::instantiate,
        cwd_core::contract::query,
    )
    .with_reply(cwd_core::contract::reply);
    Box::new(contract)
}

pub fn cwd_voting_cw4_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cwd_voting_cw4::contract::execute,
        cwd_voting_cw4::contract::instantiate,
        cwd_voting_cw4::contract::query,
    )
    .with_reply(cwd_voting_cw4::contract::reply);
    Box::new(contract)
}
