use cosmwasm_std::Empty;

use cw_multi_test::{Contract, ContractWrapper};
use cw_pre_propose_base_proposal_single as cppbps;

pub(crate) fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn cw4_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw4_group::contract::execute,
        cw4_group::contract::instantiate,
        cw4_group::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn cw721_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub(crate) fn cw20_stake_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn v1_proposal_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_proposal_single_v1::contract::execute,
        cw_proposal_single_v1::contract::instantiate,
        cw_proposal_single_v1::contract::query,
    )
    .with_reply(cw_proposal_single_v1::contract::reply)
    .with_migrate(cw_proposal_single_v1::contract::migrate);
    Box::new(contract)
}

pub(crate) fn proposal_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply)
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}

pub(crate) fn pre_propose_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cppbps::contract::execute,
        cppbps::contract::instantiate,
        cppbps::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn cw20_staked_balances_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_staked_balance_voting::contract::execute,
        cw20_staked_balance_voting::contract::instantiate,
        cw20_staked_balance_voting::contract::query,
    )
    .with_reply(cw20_staked_balance_voting::contract::reply);
    Box::new(contract)
}

pub(crate) fn native_staked_balances_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_native_staked_balance_voting::contract::execute,
        cw_native_staked_balance_voting::contract::instantiate,
        cw_native_staked_balance_voting::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn cw721_stake_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_stake::contract::execute,
        cw721_stake::contract::instantiate,
        cw721_stake::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn cw_core_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_core::contract::execute,
        cw_core::contract::instantiate,
        cw_core::contract::query,
    )
    .with_reply(cw_core::contract::reply);
    Box::new(contract)
}

pub(crate) fn cw4_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw4_voting::contract::execute,
        cw4_voting::contract::instantiate,
        cw4_voting::contract::query,
    )
    .with_reply(cw4_voting::contract::reply);
    Box::new(contract)
}
