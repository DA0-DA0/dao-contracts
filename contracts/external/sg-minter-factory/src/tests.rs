use cosmwasm_std::Addr;
use cw_multi_test::{Contract, ContractWrapper, Executor};
use sg_multi_test::StargazeApp as App;
use sg_std::StargazeMsgWrapper;

fn sg721_base_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

fn base_minter_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        base_minter::contract::execute,
        base_minter::contract::instantiate,
        base_minter::contract::query,
    )
    .with_reply(base_minter::contract::reply);
    Box::new(contract)
}

fn base_factory_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        base_factory::contract::execute,
        base_factory::contract::instantiate,
        base_factory::contract::query,
    );
    Box::new(contract)
}

fn vending_minter_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_minter::contract::execute,
        vending_minter::contract::instantiate,
        vending_minter::contract::query,
    )
    .with_reply(vending_minter::contract::reply);
    Box::new(contract)
}

fn vending_factory_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_factory::contract::execute,
        vending_factory::contract::instantiate,
        vending_factory::contract::query,
    );
    Box::new(contract)
}

fn voting_sg721_staked_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        dao_voting_cw721_staked::contract::execute,
        dao_voting_cw721_staked::contract::instantiate,
        dao_voting_cw721_staked::contract::query,
    )
    .with_reply_empty(dao_voting_cw721_staked::contract::reply);
    Box::new(contract)
}

pub struct TestEnv {
    pub app: App,
    // pub base_factory: Addr,
    pub base_minter_id: u64,
    pub sg721_base_id: u64,
    // pub vending_factory: Addr,
    pub vending_minter_id: u64,
    pub voting_: u64,
}

fn setup() -> TestEnv {
    let mut app = App::default();
    let base_factory_id = app.store_code(base_factory_contract());
    let base_minter_id = app.store_code(base_minter_contract());
    let sg721_base_id = app.store_code(sg721_base_contract());
    let vending_factory_id = app.store_code(vending_factory_contract());
    let vending_minter_id = app.store_code(vending_minter_contract());
    let voting_id = app.store_code(voting_sg721_staked_contract());

    TestEnv {
        app,
        // base_factory: base_factory_id,
        base_minter_id,
        sg721_base_id,
        // vending_factory: vending_factory_id,
        vending_minter_id,
        voting_: voting_id,
    }
}

#[test]
fn test_factory_happy_path() {
    setup();
}
