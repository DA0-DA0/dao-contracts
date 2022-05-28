use crate::contract::{instantiate, query};
use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    coin, from_binary, Addr, Decimal, FullDelegation, OwnedDeps, Uint128, Validator,
};
use cw_core_interface::voting::VotingPowerAtHeightResponse;

const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const VALI1: &str = "vali1";
const DENOM: &str = "ujuno";

fn setup_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();
    deps.querier.update_staking(
        "abcd",
        &[Validator {
            address: VALI1.to_string(),
            commission: Decimal::percent(5),
            max_commission: Decimal::percent(5),
            max_change_rate: Decimal::percent(5),
        }],
        &[
            FullDelegation {
                delegator: Addr::unchecked(ADDR1),
                validator: VALI1.to_string(),
                amount: coin(200, DENOM),
                can_redelegate: coin(200, DENOM),
                accumulated_rewards: vec![],
            },
            FullDelegation {
                delegator: Addr::unchecked(ADDR2),
                validator: VALI1.to_string(),
                amount: coin(100, DENOM),
                can_redelegate: coin(100, DENOM),
                accumulated_rewards: vec![],
            },
        ],
    );
    deps
}

#[test]
fn test_instantiate() {
    let info = mock_info(ADDR1, &[]);
    let env = mock_env();
    let mut deps = setup_deps();
    let _res = instantiate(
        deps.as_mut(),
        env,
        info,
        InstantiateMsg {
            token_denom: DENOM.to_string(),
        },
    )
    .unwrap();
}

#[test]
fn test_query_voting_power_at_height() {
    let info = mock_info(ADDR1, &[]);
    let env = mock_env();
    let mut deps = setup_deps();

    let _res = instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            token_denom: DENOM.to_string(),
        },
    )
    .unwrap();

    let bin = query(
        deps.as_ref(),
        env,
        QueryMsg::VotingPowerAtHeight {
            address: ADDR1.to_string(),
            height: None,
        },
    )
    .unwrap();
    let res: VotingPowerAtHeightResponse = from_binary(&bin).unwrap();
    assert_eq!(res.power, Uint128::new(200));
}
