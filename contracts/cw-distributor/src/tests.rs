// use cosmwasm_std::{Addr, Empty, Uint128};
// use cw2::ContractVersion;
// use cw_core_interface::voting::{
//     InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
// };
// use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};

// use crate::{
//     msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
//     ContractError,
// };

// const DAO_ADDR: &str = "dao";
// const ADDR1: &str = "addr1";
// const ADDR2: &str = "addr2";
// const ADDR3: &str = "addr3";

// fn cw4_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         cw4_group::contract::execute,
//         cw4_group::contract::instantiate,
//         cw4_group::contract::query,
//     );
//     Box::new(contract)
// }

// fn voting_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         crate::contract::execute,
//         crate::contract::instantiate,
//         crate::contract::query,
//     )
//     .with_reply(crate::contract::reply);
//     Box::new(contract)
// }

// fn instantiate_voting(app: &mut App, voting_id: u64, msg: InstantiateMsg) -> Addr {
//     app.instantiate_contract(
//         voting_id,
//         Addr::unchecked(DAO_ADDR),
//         &msg,
//         &[],
//         "voting module",
//         None,
//     )
//     .unwrap()
// }

// fn setup_test_case(app: &mut App) -> Addr {
//     let cw4_id = app.store_code(cw4_contract());
//     let voting_id = app.store_code(voting_contract());

//     let members = vec![
//         cw4::Member {
//             addr: ADDR1.to_string(),
//             weight: 1,
//         },
//         cw4::Member {
//             addr: ADDR2.to_string(),
//             weight: 1,
//         },
//         cw4::Member {
//             addr: ADDR3.to_string(),
//             weight: 1,
//         },
//     ];
//     instantiate_voting(
//         app,
//         voting_id,
//         InstantiateMsg {
//             cw4_group_code_id: cw4_id,
//             initial_members: members,
//         },
//     )
// }

// #[test]
// fn test_instantiate() {
//     let mut app = App::default();
//     // Valid instantiate no panics
//     let _voting_addr = setup_test_case(&mut app);

//     // Instantiate with no members, error
//     let voting_id = app.store_code(voting_contract());
//     let cw4_id = app.store_code(cw4_contract());
//     let msg = InstantiateMsg {
//         cw4_group_code_id: cw4_id,
//         initial_members: vec![],
//     };
//     let _err = app
//         .instantiate_contract(
//             voting_id,
//             Addr::unchecked(DAO_ADDR),
//             &msg,
//             &[],
//             "voting module",
//             None,
//         )
//         .unwrap_err();

//     // Instantiate with members but no weight
//     let msg = InstantiateMsg {
//         cw4_group_code_id: cw4_id,
//         initial_members: vec![
//             cw4::Member {
//                 addr: ADDR1.to_string(),
//                 weight: 0,
//             },
//             cw4::Member {
//                 addr: ADDR2.to_string(),
//                 weight: 0,
//             },
//             cw4::Member {
//                 addr: ADDR3.to_string(),
//                 weight: 0,
//             },
//         ],
//     };
//     let _err = app
//         .instantiate_contract(
//             voting_id,
//             Addr::unchecked(DAO_ADDR),
//             &msg,
//             &[],
//             "voting module",
//             None,
//         )
//         .unwrap_err();
// }

// #[test]
// fn test_contract_info() {
//     let mut app = App::default();
//     let voting_addr = setup_test_case(&mut app);

//     let info: InfoResponse = app
//         .wrap()
//         .query_wasm_smart(voting_addr.clone(), &QueryMsg::Info {})
//         .unwrap();
//     assert_eq!(
//         info,
//         InfoResponse {
//             info: ContractVersion {
//                 contract: "crates.io:cw4-voting".to_string(),
//                 version: env!("CARGO_PKG_VERSION").to_string()
//             }
//         }
//     );

//     // Ensure group contract is set
//     let _group_contract: Addr = app
//         .wrap()
//         .query_wasm_smart(voting_addr.clone(), &QueryMsg::GroupContract {})
//         .unwrap();

//     let dao_contract: Addr = app
//         .wrap()
//         .query_wasm_smart(voting_addr, &QueryMsg::Dao {})
//         .unwrap();
//     assert_eq!(dao_contract, Addr::unchecked(DAO_ADDR));
// }

// #[test]
// fn test_permissions() {
//     let mut app = App::default();
//     let voting_addr = setup_test_case(&mut app);

//     // DAO can not execute hook message.
//     let err: ContractError = app
//         .execute_contract(
//             Addr::unchecked(DAO_ADDR),
//             voting_addr.clone(),
//             &ExecuteMsg::MemberChangedHook { diffs: vec![] },
//             &[],
//         )
//         .unwrap_err()
//         .downcast()
//         .unwrap();
//     assert!(matches!(err, ContractError::Unauthorized {}));

//     // Contract itself can not execute hook message.
//     let err: ContractError = app
//         .execute_contract(
//             voting_addr.clone(),
//             voting_addr,
//             &ExecuteMsg::MemberChangedHook { diffs: vec![] },
//             &[],
//         )
//         .unwrap_err()
//         .downcast()
//         .unwrap();
//     assert!(matches!(err, ContractError::Unauthorized {}));
// }

// #[test]
// fn test_power_at_height() {
//     let mut app = App::default();
//     let voting_addr = setup_test_case(&mut app);
//     app.update_block(next_block);

//     let cw4_addr: Addr = app
//         .wrap()
//         .query_wasm_smart(voting_addr.clone(), &QueryMsg::GroupContract {})
//         .unwrap();

//     let addr1_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR1.to_string(),
//                 height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(addr1_voting_power.power, Uint128::new(1u128));
//     assert_eq!(addr1_voting_power.height, app.block_info().height);

//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight { height: None },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(3u128));
//     assert_eq!(total_voting_power.height, app.block_info().height);

//     // Update ADDR1's weight to 2
//     let msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
//         remove: vec![],
//         add: vec![cw4::Member {
//             addr: ADDR1.to_string(),
//             weight: 2,
//         }],
//     };

//     // Should still be one as voting power should not update until
//     // the following block.
//     let addr1_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR1.to_string(),
//                 height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(addr1_voting_power.power, Uint128::new(1u128));

//     // Same should be true about the groups contract.
//     let cw4_power: cw4::MemberResponse = app
//         .wrap()
//         .query_wasm_smart(
//             cw4_addr.clone(),
//             &cw4::Cw4QueryMsg::Member {
//                 addr: ADDR1.to_string(),
//                 at_height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(cw4_power.weight.unwrap(), 1);

//     app.execute_contract(Addr::unchecked(DAO_ADDR), cw4_addr.clone(), &msg, &[])
//         .unwrap();
//     app.update_block(next_block);

//     // Should now be 2
//     let addr1_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR1.to_string(),
//                 height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(addr1_voting_power.power, Uint128::new(2u128));
//     assert_eq!(addr1_voting_power.height, app.block_info().height);

//     // Check we can still get the 1 weight he had last block
//     let addr1_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR1.to_string(),
//                 height: Some(app.block_info().height - 1),
//             },
//         )
//         .unwrap();
//     assert_eq!(addr1_voting_power.power, Uint128::new(1u128));
//     assert_eq!(addr1_voting_power.height, app.block_info().height - 1);

//     // Check total power is now 4
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight { height: None },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(4u128));
//     assert_eq!(total_voting_power.height, app.block_info().height);

//     // Check total power for last block is 3
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight {
//                 height: Some(app.block_info().height - 1),
//             },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(3u128));
//     assert_eq!(total_voting_power.height, app.block_info().height - 1);

//     // Update ADDR1's weight back to 1
//     let msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
//         remove: vec![],
//         add: vec![cw4::Member {
//             addr: ADDR1.to_string(),
//             weight: 1,
//         }],
//     };

//     app.execute_contract(Addr::unchecked(DAO_ADDR), cw4_addr.clone(), &msg, &[])
//         .unwrap();
//     app.update_block(next_block);

//     // Should now be 1 again
//     let addr1_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR1.to_string(),
//                 height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(addr1_voting_power.power, Uint128::new(1u128));
//     assert_eq!(addr1_voting_power.height, app.block_info().height);

//     // Check total power for current block is now 3
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight { height: None },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(3u128));
//     assert_eq!(total_voting_power.height, app.block_info().height);

//     // Check total power for last block is 4
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight {
//                 height: Some(app.block_info().height - 1),
//             },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(4u128));
//     assert_eq!(total_voting_power.height, app.block_info().height - 1);

//     // Remove address 2 completely
//     let msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
//         remove: vec![ADDR2.to_string()],
//         add: vec![],
//     };

//     app.execute_contract(Addr::unchecked(DAO_ADDR), cw4_addr.clone(), &msg, &[])
//         .unwrap();
//     app.update_block(next_block);

//     // ADDR2 power is now 0
//     let addr2_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR2.to_string(),
//                 height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(addr2_voting_power.power, Uint128::zero());
//     assert_eq!(addr2_voting_power.height, app.block_info().height);

//     // Check total power for current block is now 2
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight { height: None },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(2u128));
//     assert_eq!(total_voting_power.height, app.block_info().height);

//     // Check total power for last block is 3
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight {
//                 height: Some(app.block_info().height - 1),
//             },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(3u128));
//     assert_eq!(total_voting_power.height, app.block_info().height - 1);

//     // Readd ADDR2 with 10 power
//     let msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
//         remove: vec![],
//         add: vec![cw4::Member {
//             addr: ADDR2.to_string(),
//             weight: 10,
//         }],
//     };

//     app.execute_contract(Addr::unchecked(DAO_ADDR), cw4_addr, &msg, &[])
//         .unwrap();
//     app.update_block(next_block);

//     // ADDR2 power is now 10
//     let addr2_voting_power: VotingPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::VotingPowerAtHeight {
//                 address: ADDR2.to_string(),
//                 height: None,
//             },
//         )
//         .unwrap();
//     assert_eq!(addr2_voting_power.power, Uint128::new(10u128));
//     assert_eq!(addr2_voting_power.height, app.block_info().height);

//     // Check total power for current block is now 12
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr.clone(),
//             &QueryMsg::TotalPowerAtHeight { height: None },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(12u128));
//     assert_eq!(total_voting_power.height, app.block_info().height);

//     // Check total power for last block is 2
//     let total_voting_power: TotalPowerAtHeightResponse = app
//         .wrap()
//         .query_wasm_smart(
//             voting_addr,
//             &QueryMsg::TotalPowerAtHeight {
//                 height: Some(app.block_info().height - 1),
//             },
//         )
//         .unwrap();
//     assert_eq!(total_voting_power.power, Uint128::new(2u128));
//     assert_eq!(total_voting_power.height, app.block_info().height - 1);
// }
