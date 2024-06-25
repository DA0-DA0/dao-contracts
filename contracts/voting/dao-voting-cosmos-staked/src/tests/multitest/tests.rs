use cosmwasm_std::{
    coin, coins, testing::mock_env, Addr, CosmosMsg, Decimal, Empty, StakingMsg, Uint128, Validator,
};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::msg::{InstantiateMsg, QueryMsg};

use super::app::CustomApp;

fn cosmos_staked_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

const DAO: &str = "dao";
pub const DELEGATOR: &str = "delegator";
pub const VALIDATOR: &str = "validator";
const VALIDATOR2: &str = "validator2";
const DENOM: &str = "TOKEN";

fn setup_test_env() -> CustomApp {
    CustomApp::new(|router, api, storage| {
        router
            .staking
            .add_validator(
                api,
                storage,
                &mock_env().block,
                Validator {
                    address: VALIDATOR.to_string(),
                    commission: Decimal::zero(), // zero percent comission to keep math simple.
                    max_commission: Decimal::percent(10),
                    max_change_rate: Decimal::percent(2),
                },
            )
            .unwrap();
        router
            .staking
            .add_validator(
                api,
                storage,
                &mock_env().block,
                Validator {
                    address: VALIDATOR2.to_string(),
                    commission: Decimal::zero(), // zero percent comission to keep math simple.
                    max_commission: Decimal::percent(10),
                    max_change_rate: Decimal::percent(2),
                },
            )
            .unwrap();
        router
            .bank
            .init_balance(storage, &Addr::unchecked(DELEGATOR), coins(1000000, DENOM))
            .unwrap();
    })
}

#[test]
fn happy_path() {
    let mut app = setup_test_env();

    let cosmos_staking_code_id = app.store_code(cosmos_staked_contract());

    let vp_contract = app
        .instantiate_contract(
            cosmos_staking_code_id,
            Addr::unchecked(DAO),
            &InstantiateMsg {},
            &[],
            "cosmos_voting_power_contract",
            None,
        )
        .unwrap();

    // Stake!
    app.execute(
        Addr::unchecked(DELEGATOR),
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount: coin(100000, DENOM),
        }),
    )
    .unwrap();

    // Update block height
    app.update_block(|block| block.height += 1);

    // Query voting power
    let vp: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            vp_contract.clone(),
            &QueryMsg::VotingPowerAtHeight {
                height: Some(12346),
                address: DELEGATOR.to_string(),
            },
        )
        .unwrap();

    // Check amounts
    assert_eq!(vp.power, Uint128::new(100000));

    // Query total voting power
    let tp: TotalPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            vp_contract,
            &QueryMsg::TotalPowerAtHeight {
                height: Some(12346),
            },
        )
        .unwrap();

    // Check totals
    assert_eq!(tp.power, Uint128::new(100000));
}

#[test]
fn test_query_dao() {
    let mut app = setup_test_env();

    let cosmos_staking_code_id = app.store_code(cosmos_staked_contract());

    let addr = app
        .instantiate_contract(
            cosmos_staking_code_id,
            Addr::unchecked(DAO),
            &InstantiateMsg {},
            &[],
            "cosmos_voting_power_contract",
            None,
        )
        .unwrap();

    let dao: Addr = app
        .wrap()
        .query_wasm_smart(addr, &QueryMsg::Dao {})
        .unwrap();
    assert_eq!(dao, Addr::unchecked(DAO));
}

#[test]
fn test_query_info() {
    let mut app = setup_test_env();

    let cosmos_staking_code_id = app.store_code(cosmos_staked_contract());

    let addr = app
        .instantiate_contract(
            cosmos_staking_code_id,
            Addr::unchecked(DAO),
            &InstantiateMsg {},
            &[],
            "cosmos_voting_power_contract",
            None,
        )
        .unwrap();

    let resp: InfoResponse = app
        .wrap()
        .query_wasm_smart(addr, &QueryMsg::Info {})
        .unwrap();
    assert_eq!(resp.info.contract, "crates.io:dao-voting-cosmos-staked");
}
