use crate::contract::{instantiate, query};
use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, from_binary, Addr, Decimal, FullDelegation, Uint128, Validator};
use cw_core_interface::voting::VotingPowerAtHeightResponse;

#[test]
fn test_instantiate() {
    let info = mock_info("addr1", &[]);
    let env = mock_env();
    let mut deps = mock_dependencies();
    let _res = instantiate(
        deps.as_mut(),
        env,
        info,
        InstantiateMsg {
            token_denom: "abcd".to_string(),
        },
    )
    .unwrap();
}

#[test]
fn test_query_voting_power_at_height() {
    let info = mock_info("addr1", &[]);
    let env = mock_env();
    let mut deps = mock_dependencies();
    deps.querier.update_staking(
        "abcd",
        &[Validator {
            address: "vali1".to_string(),
            commission: Decimal::percent(5),
            max_commission: Decimal::percent(5),
            max_change_rate: Decimal::percent(5),
        }],
        &[FullDelegation {
            delegator: Addr::unchecked("addr1"),
            validator: "vali1".to_string(),
            amount: coin(200, "abcd"),
            can_redelegate: coin(200, "abcd"),
            accumulated_rewards: vec![],
        }],
    );

    let _res = instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            token_denom: "abcd".to_string(),
        },
    )
    .unwrap();

    let bin = query(
        deps.as_ref(),
        env,
        QueryMsg::VotingPowerAtHeight {
            address: "addr1".to_string(),
            height: None,
        },
    )
    .unwrap();
    let res: VotingPowerAtHeightResponse = from_binary(&bin).unwrap();
    assert_eq!(res.power, Uint128::new(200));
}
