use cosmwasm_std::{
    coin, coins, testing::mock_env, to_json_binary, Addr, CosmosMsg, Decimal, Empty, StakingMsg,
    Uint128, Validator,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, SudoMsg, WasmSudo};
use dao_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};

use crate::msg::{InstantiateMsg, QueryMsg};

fn cosmos_staking_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_sudo(crate::contract::sudo);
    Box::new(contract)
}

const DAO: &str = "dao";
const DELEGATOR: &str = "delegator";
const VALIDATOR: &str = "validator";
const VALIDATOR2: &str = "validator2";
const DENOM: &str = "TOKEN";

fn setup_test_env() -> App {
    App::new(|router, api, storage| {
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

    let cosmos_staking_code_id = app.store_code(cosmos_staking_contract());

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

    // Manually update a delegation, normally this would be called by cw-hooks
    app.sudo(SudoMsg::Wasm(WasmSudo {
        contract_addr: vp_contract.clone(),
        msg: to_json_binary(&crate::msg::SudoMsg::AfterDelegationModified {
            validator_address: VALIDATOR.to_string(),
            delegator_address: DELEGATOR.to_string(),
            shares: "100000".to_string(),
        })
        .unwrap(),
    }))
    .unwrap();

    println!("{:?}", app.block_info());

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
