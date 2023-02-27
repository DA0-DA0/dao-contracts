// use std::{str::FromStr, time::Duration};

// use cosm_orc::orchestrator::{cosm_orc::tokio_block, Address, Coin, Denom};
// use cosm_tome::clients::client::{CosmTome, CosmosClient};
// use cw_vesting::{
//     msg::{ExecuteMsg, InstantiateMsg},
//     vesting::Schedule,
// };

// use cosmwasm_std::Uint128;
use test_context::test_context;

use crate::helpers::chain::Chain;

// const CONTRACT_NAME: &str = "cw_vesting";
// // junod query staking validators on the juno docker node
// const VALIDATOR: &str = "junovaloper16mzxzn5xcrgj7jun0wmggy49ksl7glzgplg8z3";

// async fn balance<C: CosmosClient>(addr: &str, client: &CosmTome<C>) -> u128 {
//     client
//         .bank_query_balance(
//             Address::from_str(addr).unwrap(),
//             Denom::from_str("ujunox").unwrap(),
//         )
//         .await
//         .unwrap()
//         .balance
//         .amount
// }

// TODO CHECK INVALID ADDRESS (UN)DELEGATION ERRORS

// TODO THIS TEST DOES NOT CURRENTLY WORK BECAUSE I CAN NOT WORK OUT
// HOW TO PROGRAMATICALLY GET THE VALIDATOR OPERATOR ADDRESS AND THE
// VALIDATOR ADDRESS IS NON DETERMINSITIC.

// Checks that we still can not do an unbonding duration query.
#[test_context(Chain)]
#[test]
#[ignore]
fn test_cw_vesting_staking(_chain: &mut Chain) {
    // let user_addr = chain.users["user5"].account.address.clone();
    // let user_key = chain.users["user5"].key.clone();

    // let pk = tokio_block(chain.orc.client.tendermint_query_latest_validator_set(None))
    //     .unwrap()
    //     .validators
    //     .into_iter()
    //     .next()
    //     .unwrap()
    //     .pubkey
    //     .unwrap();
    // let id = pk.account_id("junovaloper").unwrap();
    // let validator = Address::from(id).to_string();

    // // let vals = tokio_block(chain.orc.client.tendermint_query_latest_validator_set(None))
    // //     .unwrap()
    // //     .validators;
    // // panic!("{vals:#?}");

    // // let addr = tokio_block(chain.orc.client.tendermint_query_latest_validator_set(None))
    // //     .unwrap()
    // //     .validators
    // //     .into_iter()
    // //     .next()
    // //     .unwrap()
    // //     .address;
    // // let validator = Address::new("junovaloper", &addr.to_bytes())
    // //     .unwrap()
    // //     .to_string();

    // // at this point we have the bytes of the hash of the public key.
    // println!("{validator}");

    // chain
    //     .orc
    //     .instantiate(
    //         CONTRACT_NAME,
    //         "instantiate_cw_vesting",
    //         &InstantiateMsg {
    //             owner: Some(user_addr.clone()),
    //             recipient: user_addr.to_string(),

    //             title: "title".to_string(),
    //             description: Some("description".to_string()),

    //             total: Uint128::new(100),
    //             denom: cw_vesting::UncheckedDenom::Native("ujunox".to_string()),

    //             schedule: Schedule::SaturatingLinear,
    //             start_time: None,
    //             vesting_duration_seconds: 10,
    //             unbonding_duration_seconds: 2592000,
    //         },
    //         &user_key,
    //         None,
    //         vec![Coin {
    //             denom: Denom::from_str("ujunox").unwrap(),
    //             amount: 100,
    //         }],
    //     )
    //     .unwrap();

    // chain
    //     .orc
    //     .execute(
    //         CONTRACT_NAME,
    //         "stake_cw_vesting",
    //         &ExecuteMsg::Delegate {
    //             validator,
    //             amount: Uint128::new(950_000_000),
    //         },
    //         &user_key,
    //         vec![],
    //     )
    //     .unwrap();

    // chain
    //     .orc
    //     .poll_for_n_blocks(10, Duration::from_secs(100), false)
    //     .unwrap();

    // let start = tokio_block(balance(&user_addr, &chain.orc.client));

    // chain
    //     .orc
    //     .execute(
    //         CONTRACT_NAME,
    //         "stake_cw_vesting",
    //         &ExecuteMsg::WithdrawDelegatorReward {
    //             validator: VALIDATOR.to_string(),
    //         },
    //         &user_key,
    //         vec![],
    //     )
    //     .unwrap();

    // let end = tokio_block(balance(&user_addr, &chain.orc.client));

    // assert!(end > start, "{end} > {start}");
}
