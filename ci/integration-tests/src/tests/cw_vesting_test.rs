use std::str::FromStr;

use cosm_orc::orchestrator::{Coin, Denom};
use cw_vesting::{
    msg::{InstantiateMsg, QueryMsg},
    vesting::Schedule,
};

use cosmwasm_std::Uint128;
use test_context::test_context;

use crate::helpers::chain::Chain;

const CONTRACT_NAME: &str = "cw_vesting";

// Checks that we still can not do an unbonding duration query.
#[test_context(Chain)]
#[test]
#[ignore]
fn query_unbonding_duration(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();

    chain
        .orc
        .instantiate(
            CONTRACT_NAME,
            "instantiate_cw_vesting",
            &InstantiateMsg {
                owner: Some(user_addr.clone()),
                recipient: user_addr,

                title: "title".to_string(),
                description: "description".to_string(),

                total: Uint128::new(1),
                denom: cw_vesting::UncheckedDenom::Native("ujunox".to_string()),

                schedule: Schedule::SaturatingLinear,
                start_time: None,
                duration_seconds: 10,
            },
            &user_key,
            None,
            vec![Coin {
                denom: Denom::from_str("ujunox").unwrap(),
                amount: 1,
            }],
        )
        .unwrap();

    chain
        .orc
        .query(CONTRACT_NAME, &QueryMsg::UnbondingDurationSeconds {})
        .unwrap_err()
        .to_string()
        .contains("Unsupported query type: Stargate queries are disabled");
}
