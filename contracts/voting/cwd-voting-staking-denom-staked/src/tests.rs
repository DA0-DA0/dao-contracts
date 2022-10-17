use crate::contract::{instantiate, query};
use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    coin, from_binary, Addr, Coin, Deps, Env, FullDelegation, OwnedDeps, Uint128, Validator,
};
use cwd_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

const DAO_ADDR: &str = "dao";
const STAKING_MODULE_ADDR: &str = "addrstaking";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const DENOM: &str = "ujuno";
const OTHER_DENOM: &str = "uatom";

const VALI1: &str = "vali1";
const VALI2: &str = "vali2";

fn setup_deps(
    validators: Vec<Validator>,
    delegations: Vec<FullDelegation>,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();
    deps.querier
        .update_staking(DENOM, &validators, &delegations);
    deps
}

fn get_staking_module(deps: Deps, env: Env) -> Addr {
    let msg = QueryMsg::StakingModule {};
    let bin = query(deps, env, msg).unwrap();
    from_binary(&bin).unwrap()
}

fn get_dao(deps: Deps, env: Env) -> Addr {
    let msg = QueryMsg::Dao {};
    let bin = query(deps, env, msg).unwrap();
    from_binary(&bin).unwrap()
}

fn get_info(deps: Deps, env: Env) -> InfoResponse {
    let msg = QueryMsg::Info {};
    let bin = query(deps, env, msg).unwrap();
    from_binary(&bin).unwrap()
}

fn get_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> TotalPowerAtHeightResponse {
    let msg = QueryMsg::TotalPowerAtHeight { height };
    let bin = query(deps, env, msg).unwrap();
    from_binary(&bin).unwrap()
}

fn get_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: &str,
    height: Option<u64>,
) -> VotingPowerAtHeightResponse {
    let msg = QueryMsg::VotingPowerAtHeight {
        address: address.to_string(),
        height,
    };
    let bin = query(deps, env, msg).unwrap();
    from_binary(&bin).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = setup_deps(vec![], vec![]);
    let env = mock_env();
    let info = mock_info(DAO_ADDR, &[]);
    let msg = InstantiateMsg {
        staking_module_address: STAKING_MODULE_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let staking_module = get_staking_module(deps.as_ref(), env.clone());
    assert_eq!(staking_module, Addr::unchecked(STAKING_MODULE_ADDR));

    let dao = get_dao(deps.as_ref(), env.clone());
    assert_eq!(dao, Addr::unchecked(DAO_ADDR));

    let info = get_info(deps.as_ref(), env);
    assert_eq!(
        info.info.contract,
        "crates.io:cwd-voting-staking-denom-staked"
    );
}

#[test]
fn test_power_queries() {
    // Start with no delegations
    let mut deps = setup_deps(
        vec![
            Validator {
                address: VALI1.to_string(),
                commission: Default::default(),
                max_commission: Default::default(),
                max_change_rate: Default::default(),
            },
            Validator {
                address: VALI2.to_string(),
                commission: Default::default(),
                max_commission: Default::default(),
                max_change_rate: Default::default(),
            },
        ],
        vec![],
    );
    let env = mock_env();
    let info = mock_info(DAO_ADDR, &[]);

    // Setup contract
    let msg = InstantiateMsg {
        staking_module_address: STAKING_MODULE_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Query total power, should be 0
    let resp = get_total_power_at_height(deps.as_ref(), env.clone(), None);
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, env.block.height);

    // Query voting power for both addresses, should be 0
    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR1, None);
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, env.block.height);

    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR2, None);
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, env.block.height);

    // Test passing a height, our contract has no regard for height due to unbonding
    // durations, so should still return the same just a different height
    let resp = get_total_power_at_height(deps.as_ref(), env.clone(), Some(1));
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, 1);

    // Query voting power for both addresses, should be 0
    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR1, Some(1));
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, 1);

    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR2, Some(1));
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, 1);

    // Setup stakes, ADDR1 stakes both DENOM and OTHER_DENOM to VALI1 (to test our filter)
    // ADDR2 stakes just DENOM, but to both valis
    // Total power: 400 (150 + 150 + 100)
    // ADDR1 power: 100
    // ADDR2 power: 300 (150 + 150)
    deps.querier.update_staking(
        DENOM,
        &[
            Validator {
                address: VALI1.to_string(),
                commission: Default::default(),
                max_commission: Default::default(),
                max_change_rate: Default::default(),
            },
            Validator {
                address: VALI2.to_string(),
                commission: Default::default(),
                max_commission: Default::default(),
                max_change_rate: Default::default(),
            },
        ],
        &[
            // ADDR1 DENOM
            FullDelegation {
                delegator: Addr::unchecked(ADDR1),
                validator: VALI1.to_string(),
                amount: coin(100, DENOM),
                can_redelegate: Default::default(),
                accumulated_rewards: vec![],
            },
            // ADDR1 OTHER_DENOM, this is to test we ignore it
            FullDelegation {
                delegator: Addr::unchecked(ADDR1),
                validator: VALI1.to_string(),
                amount: coin(50, OTHER_DENOM),
                can_redelegate: Default::default(),
                accumulated_rewards: vec![],
            },
            // ADDR2 VALI1
            FullDelegation {
                delegator: Addr::unchecked(ADDR2),
                validator: VALI1.to_string(),
                amount: coin(150, DENOM),
                can_redelegate: Default::default(),
                accumulated_rewards: vec![],
            },
            // ADDR2 VALI2
            FullDelegation {
                delegator: Addr::unchecked(ADDR2),
                validator: VALI2.to_string(),
                amount: coin(150, DENOM),
                can_redelegate: Default::default(),
                accumulated_rewards: vec![],
            },
        ],
    );
    // Now we need to update the balance for our staking module
    // In reality the SDK handles this for us : )
    deps.querier.update_balance(
        STAKING_MODULE_ADDR,
        vec![Coin {
            denom: DENOM.to_string(),
            amount: Uint128::new(400),
        }],
    );

    // Query total power, should now be 400
    let resp = get_total_power_at_height(deps.as_ref(), env.clone(), None);
    assert_eq!(resp.power, Uint128::new(400));
    assert_eq!(resp.height, env.block.height);

    // Query voting power for both addresses
    // ADDR1 has 100 staked to one validator
    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR1, None);
    assert_eq!(resp.power, Uint128::new(100));
    assert_eq!(resp.height, env.block.height);

    // ADDR2 has 300 in total, 150 to each validator
    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR2, None);
    assert_eq!(resp.power, Uint128::new(300));
    assert_eq!(resp.height, env.block.height);

    // Spoof stake again, this time ADDR2 is not staking at all
    // Setup stakes, ADDR1 stakes both DENOM and OTHER_DENOM to VALI1 (to test our filter)
    // ADDR2 does not stake
    // Total power: 100
    // ADDR1 power: 100
    // ADDR2 power: 0
    deps.querier.update_staking(
        DENOM,
        &[
            Validator {
                address: VALI1.to_string(),
                commission: Default::default(),
                max_commission: Default::default(),
                max_change_rate: Default::default(),
            },
            Validator {
                address: VALI2.to_string(),
                commission: Default::default(),
                max_commission: Default::default(),
                max_change_rate: Default::default(),
            },
        ],
        &[
            // ADDR1 DENOM
            FullDelegation {
                delegator: Addr::unchecked(ADDR1),
                validator: VALI1.to_string(),
                amount: coin(100, DENOM),
                can_redelegate: Default::default(),
                accumulated_rewards: vec![],
            },
            // ADDR1 OTHER_DENOM, this is to test we ignore it
            FullDelegation {
                delegator: Addr::unchecked(ADDR1),
                validator: VALI1.to_string(),
                amount: coin(50, OTHER_DENOM),
                can_redelegate: Default::default(),
                accumulated_rewards: vec![],
            },
        ],
    );
    // Now we need to update the balance for our staking module
    // In reality the SDK handles this for us : )
    deps.querier.update_balance(
        STAKING_MODULE_ADDR,
        vec![Coin {
            denom: DENOM.to_string(),
            amount: Uint128::new(100),
        }],
    );

    // Query total power, should now be 100
    let resp = get_total_power_at_height(deps.as_ref(), env.clone(), None);
    assert_eq!(resp.power, Uint128::new(100));
    assert_eq!(resp.height, env.block.height);

    // Query voting power for both addresses
    // ADDR1 has 100 staked to one validator
    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR1, None);
    assert_eq!(resp.power, Uint128::new(100));
    assert_eq!(resp.height, env.block.height);

    // ADDR2 has 0 again
    let resp = get_voting_power_at_height(deps.as_ref(), env.clone(), ADDR2, None);
    assert_eq!(resp.power, Uint128::zero());
    assert_eq!(resp.height, env.block.height);
}
