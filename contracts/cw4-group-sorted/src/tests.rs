use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    from_binary, from_slice, Api, Deps, DepsMut, OwnedDeps, Querier, Response, Storage, SubMsg,
};
use cosmwasm_std::{Addr, Empty};
use cw4::{
    member_key, MemberChangedHookMsg, MemberDiff, MemberResponse, TotalWeightResponse, TOTAL_KEY,
};
use cw_controllers::{AdminError, HookError};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::contract::{
    execute, execute_update_members, instantiate, query_member, query_total_weight,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{AdminResponse, Member, MemberListResponse};
use crate::state::{list_members_sorted, ADMIN, HOOKS};
use crate::ContractError;

const INIT_ADMIN: &str = "juan";
const USER1: &str = "somebody";
const USER2: &str = "else";
const USER3: &str = "funny";

fn address_list_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn setup_test_case(app: &mut App, admin: Option<String>, members: Vec<Member>) -> Addr {
    let address_list_id = app.store_code(address_list_contract());
    app.instantiate_contract(
        address_list_id,
        Addr::unchecked(INIT_ADMIN),
        &crate::msg::InstantiateMsg { admin, members },
        &[],
        "address-manager",
        None,
    )
    .unwrap()
}

fn generate_member(weight: u64) -> Member {
    Member {
        addr: Addr::unchecked(format!("addr_{}", weight)),
        weight,
    }
}

#[test]
fn test_instantiate_with_admin() {
    let mut app = App::default();
    let contract = setup_test_case(&mut app, Some(INIT_ADMIN.to_string()), vec![]);

    let admin: AdminResponse = app
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(admin.admin, Some(INIT_ADMIN.to_string()))
}

#[test]
fn test_instantiate_without_admin() {
    let mut app = App::default();
    let contract = setup_test_case(&mut app, None, vec![]);

    let admin: AdminResponse = app
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(admin.admin, None)
}

#[test]
fn test_query_members() {
    let mut app = App::default();

    let members = (1..10).rev().map(generate_member).collect::<Vec<Member>>();
    let members_reversed = members.iter().rev().cloned().collect::<Vec<Member>>();

    let contract = setup_test_case(&mut app, None, members_reversed);

    let member_list: MemberListResponse = app
        .wrap()
        .query_wasm_smart(
            contract,
            &QueryMsg::ListMembers {
                limit: None,
                start_after: None,
            },
        )
        .unwrap();
    assert_eq!(member_list.members, members)
}

fn do_query_total_weight(deps: Deps) -> TotalWeightResponse {
    from_binary(&query_total_weight(deps).unwrap()).unwrap()
}
fn do_query_member(deps: Deps, who: &str, height: Option<u64>) -> MemberResponse {
    from_binary(&query_member(deps, who.to_string(), height).unwrap()).unwrap()
}
fn do_update_members(
    deps: DepsMut,
    height: u64,
    sender: Addr,
    add: Vec<Member>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    let info = mock_info(sender.as_str(), &[]);
    execute_update_members(deps, info, add, remove, height)
}

fn do_instantiate(deps: DepsMut) {
    let msg = InstantiateMsg {
        admin: Some(INIT_ADMIN.into()),
        members: vec![
            Member {
                addr: Addr::unchecked(USER1),
                weight: 11,
            },
            Member {
                addr: Addr::unchecked(USER2),
                weight: 6,
            },
        ],
    };
    let info = mock_info("creator", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap();
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    // it worked, let's query the state
    let res = ADMIN.query_admin(deps.as_ref()).unwrap();
    assert_eq!(Some(INIT_ADMIN.into()), res.admin);

    let res = do_query_total_weight(deps.as_ref());
    assert_eq!(17, res.weight);
}

#[test]
fn try_member_queries() {
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    let member1 = do_query_member(deps.as_ref(), USER1.into(), None);
    assert_eq!(member1.weight, Some(11));

    let member2 = do_query_member(deps.as_ref(), USER2.into(), None);
    assert_eq!(member2.weight, Some(6));

    let member3 = do_query_member(deps.as_ref(), USER3.into(), None);
    assert_eq!(member3.weight, None);

    let members = list_members_sorted(&deps.storage).unwrap();
    assert_eq!(members.len(), 2);
}

fn assert_users<S: Storage, A: Api, Q: Querier>(
    deps: &OwnedDeps<S, A, Q>,
    user1_weight: Option<u64>,
    user2_weight: Option<u64>,
    user3_weight: Option<u64>,
    height: Option<u64>,
) {
    let member1 = do_query_member(deps.as_ref(), USER1.into(), height);
    assert_eq!(member1.weight, user1_weight);

    let member2 = do_query_member(deps.as_ref(), USER2.into(), height);
    assert_eq!(member2.weight, user2_weight);

    let member3 = do_query_member(deps.as_ref(), USER3.into(), height);
    assert_eq!(member3.weight, user3_weight);

    // this is only valid if we are not doing a historical query
    if height.is_none() {
        // compute expected metrics
        let weights = vec![user1_weight, user2_weight, user3_weight];
        let sum: u64 = weights.iter().map(|x| x.unwrap_or_default()).sum();
        let count = weights.iter().filter(|x| x.is_some()).count();

        // TODO: more detailed compare?
        let members = list_members_sorted(&deps.storage).unwrap();
        assert_eq!(count, members.len());

        let total = do_query_total_weight(deps.as_ref());
        assert_eq!(sum, total.weight); // 17 - 11 + 15 = 21
    }
}

#[test]
fn add_new_remove_old_member() {
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    // add a new one and remove existing one
    let add = vec![Member {
        addr: Addr::unchecked(USER3),
        weight: 15,
    }];
    let remove = vec![USER1.into()];

    // non-admin cannot update
    let height = mock_env().block.height;
    let err = do_update_members(
        deps.as_mut(),
        height + 5,
        Addr::unchecked(USER1),
        add.clone(),
        remove.clone(),
    )
    .unwrap_err();
    assert_eq!(err, AdminError::NotAdmin {}.into());

    // Test the values from instantiate
    assert_users(&deps, Some(11), Some(6), None, None);
    // Note all values were set at height, the beginning of that block was all None
    assert_users(&deps, None, None, None, Some(height));
    // This will get us the values at the start of the block after instantiate (expected initial values)
    assert_users(&deps, Some(11), Some(6), None, Some(height + 1));

    // admin updates properly
    do_update_members(
        deps.as_mut(),
        height + 10,
        Addr::unchecked(INIT_ADMIN),
        add,
        remove,
    )
    .unwrap();

    // updated properly
    assert_users(&deps, None, Some(6), Some(15), None);

    // snapshot still shows old value
    assert_users(&deps, Some(11), Some(6), None, Some(height + 1));
}

#[test]
fn add_old_remove_new_member() {
    // add will over-write and remove have no effect
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    // add a new one and remove existing one
    let add = vec![Member {
        addr: Addr::unchecked(USER1),
        weight: 4,
    }];
    let remove = vec![USER3.into()];

    // admin updates properly
    let height = mock_env().block.height;
    do_update_members(
        deps.as_mut(),
        height,
        Addr::unchecked(INIT_ADMIN),
        add,
        remove,
    )
    .unwrap();
    assert_users(&deps, Some(4), Some(6), None, None);
}

#[test]
fn add_and_remove_same_member() {
    // add will over-write and remove have no effect
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    // USER1 is updated and remove in the same call, we should remove this an add member3
    let add = vec![
        Member {
            addr: Addr::unchecked(USER1),
            weight: 20,
        },
        Member {
            addr: Addr::unchecked(USER3),
            weight: 5,
        },
    ];
    let remove = vec![USER1.into()];

    // admin updates properly
    let height = mock_env().block.height;
    do_update_members(
        deps.as_mut(),
        height,
        Addr::unchecked(INIT_ADMIN),
        add,
        remove,
    )
    .unwrap();
    assert_users(&deps, None, Some(6), Some(5), None);
}

#[test]
fn add_remove_hooks() {
    // add will over-write and remove have no effect
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
    assert!(hooks.hooks.is_empty());

    let contract1 = String::from("hook1");
    let contract2 = String::from("hook2");

    let add_msg = ExecuteMsg::AddHook {
        addr: contract1.clone(),
    };

    // non-admin cannot add hook
    let user_info = mock_info(USER1, &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        user_info.clone(),
        add_msg.clone(),
    )
    .unwrap_err();
    assert_eq!(err, HookError::Admin(AdminError::NotAdmin {}).into());

    // admin can add it, and it appears in the query
    let admin_info = mock_info(INIT_ADMIN, &[]);
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        admin_info.clone(),
        add_msg.clone(),
    )
    .unwrap();
    let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
    assert_eq!(hooks.hooks, vec![contract1.clone()]);

    // cannot remove a non-registered contract
    let remove_msg = ExecuteMsg::RemoveHook {
        addr: contract2.clone(),
    };
    let err = execute(deps.as_mut(), mock_env(), admin_info.clone(), remove_msg).unwrap_err();
    assert_eq!(err, HookError::HookNotRegistered {}.into());

    // add second contract
    let add_msg2 = ExecuteMsg::AddHook {
        addr: contract2.clone(),
    };
    let _ = execute(deps.as_mut(), mock_env(), admin_info.clone(), add_msg2).unwrap();
    let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
    assert_eq!(hooks.hooks, vec![contract1.clone(), contract2.clone()]);

    // cannot re-add an existing contract
    let err = execute(deps.as_mut(), mock_env(), admin_info.clone(), add_msg).unwrap_err();
    assert_eq!(err, HookError::HookAlreadyRegistered {}.into());

    // non-admin cannot remove
    let remove_msg = ExecuteMsg::RemoveHook { addr: contract1 };
    let err = execute(deps.as_mut(), mock_env(), user_info, remove_msg.clone()).unwrap_err();
    assert_eq!(err, HookError::Admin(AdminError::NotAdmin {}).into());

    // remove the original
    let _ = execute(deps.as_mut(), mock_env(), admin_info, remove_msg).unwrap();
    let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
    assert_eq!(hooks.hooks, vec![contract2]);
}

#[test]
fn hooks_fire() {
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
    assert!(hooks.hooks.is_empty());

    let contract1 = String::from("hook1");
    let contract2 = String::from("hook2");

    // register 2 hooks
    let admin_info = mock_info(INIT_ADMIN, &[]);
    let add_msg = ExecuteMsg::AddHook {
        addr: contract1.clone(),
    };
    let add_msg2 = ExecuteMsg::AddHook {
        addr: contract2.clone(),
    };
    for msg in vec![add_msg, add_msg2] {
        let _ = execute(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
    }

    // make some changes - add 3, remove 2, and update 1
    // USER1 is updated and remove in the same call, we should remove this an add member3
    let add = vec![
        Member {
            addr: Addr::unchecked(USER1),
            weight: 20,
        },
        Member {
            addr: Addr::unchecked(USER3),
            weight: 5,
        },
    ];
    let remove = vec![USER2.into()];
    let msg = ExecuteMsg::UpdateMembers { remove, add };

    // admin updates properly
    assert_users(&deps, Some(11), Some(6), None, None);
    let res = execute(deps.as_mut(), mock_env(), admin_info, msg).unwrap();
    assert_users(&deps, Some(20), None, Some(5), None);

    // ensure 2 messages for the 2 hooks
    assert_eq!(res.messages.len(), 2);
    // same order as in the message (adds first, then remove)
    let diffs = vec![
        MemberDiff::new(USER1, Some(11), Some(20)),
        MemberDiff::new(USER3, None, Some(5)),
        MemberDiff::new(USER2, Some(6), None),
    ];
    let hook_msg = MemberChangedHookMsg { diffs };
    let msg1 = SubMsg::new(hook_msg.clone().into_cosmos_msg(contract1).unwrap());
    let msg2 = SubMsg::new(hook_msg.into_cosmos_msg(contract2).unwrap());
    assert_eq!(res.messages, vec![msg1, msg2]);
}

#[test]
fn raw_queries_work() {
    // add will over-write and remove have no effect
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());

    // get total from raw key
    let total_raw = deps.storage.get(TOTAL_KEY.as_bytes()).unwrap();
    let total: u64 = from_slice(&total_raw).unwrap();
    assert_eq!(17, total);

    // get member votes from raw key
    let member2_raw = deps.storage.get(&member_key(USER2)).unwrap();
    let member2: u64 = from_slice(&member2_raw).unwrap();
    assert_eq!(6, member2);

    // and execute misses
    let member3_raw = deps.storage.get(&member_key(USER3));
    assert_eq!(None, member3_raw);
}
