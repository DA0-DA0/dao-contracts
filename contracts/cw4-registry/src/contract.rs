#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListGroupsResponse, QueryMsg};
use crate::state::{EMPTY, MEMBER_INDEX};

use cw2::set_contract_version;
use cw4::MemberChangedHookMsg;
use cw4_group::helpers::Cw4GroupContract;
use cw_storage_plus::Bound;

const CONTRACT_NAME: &str = "crates.io:cw4-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register { group_addrs } => execute_register(deps, env, info, group_addrs),
        ExecuteMsg::MemberChangedHook(msg) => execute_member_changed_hook(deps, env, info, msg),
    }
}

pub fn execute_register(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    group_addrs: Vec<String>,
) -> Result<Response, ContractError> {
    // register groups
    for addr in group_addrs {
        let group_addr = deps.api.addr_validate(&addr)?;

        let contract = Cw4GroupContract::new(group_addr.clone());
        // is registered as hook?
        if !contract
            .hooks(&deps.querier)?
            .contains(&env.contract.address.clone().into_string())
        {
            return Err(ContractError::Unauthorized {});
        }

        let members = contract.list_members(&deps.querier, None, None)?;

        for m in members {
            let member_addr = deps.api.addr_validate(m.addr.as_str())?;
            MEMBER_INDEX.save(deps.storage, (&member_addr, &group_addr), &EMPTY)?;
        }
    }

    Ok(Response::default())
}

pub fn execute_member_changed_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    let group_addr = info.sender;
    for md in msg.diffs {
        // add new addresses
        if md.new.is_some() {
            let key = deps.api.addr_validate(md.key.as_str())?;
            MEMBER_INDEX.save(deps.storage, (&key, &group_addr), &EMPTY)?;
        }

        // remove old addresses
        if md.old.is_some() {
            let key = deps.api.addr_validate(md.key.as_str())?;
            MEMBER_INDEX.remove(deps.storage, (&key, &group_addr));
        }
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ListGroups {
            user_addr,
            start_after,
            limit,
        } => to_binary(&query_groups(deps, user_addr, start_after, limit)?),
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_groups(
    deps: Deps,
    user_addr: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListGroupsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(user_addr.as_str())?;
    let start_after = start_after.map(Bound::inclusive);

    let groups: StdResult<Vec<Addr>> = MEMBER_INDEX
        .prefix_de(&addr)
        .keys_de(deps.storage, start_after, None, Order::Ascending)
        .take(limit)
        .collect();

    let groups_str = groups?.into_iter().map(|g| g.into_string()).collect();

    Ok(ListGroupsResponse { groups: groups_str })
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use crate::ContractError;
    use anyhow::Error;

    use crate::helpers::Cw4RegistryContract;
    use assert_matches::assert_matches;
    use cosmwasm_std::{to_binary, Addr, Empty, WasmMsg};
    use cw4::Member;
    use cw4_group::helpers::Cw4GroupContract;
    use cw_multi_test::{App, BasicApp, Contract, ContractWrapper, Executor};

    fn mock_app() -> App {
        App::default()
    }

    pub fn contract_cw4_registry() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    pub fn contract_cw4_group() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw4_group::contract::execute,
            cw4_group::contract::instantiate,
            cw4_group::contract::query,
        );
        Box::new(contract)
    }

    const ADMIN_ADDR: &str = "admin";
    const ADDR1: &str = "add1";
    const ADDR2: &str = "add2";
    const ADDR3: &str = "add3";
    const ADDR6: &str = "add6";
    const ADDR7: &str = "add7";

    fn setup_environment(
        router: &mut BasicApp,
    ) -> ((Cw4GroupContract, Cw4GroupContract), Cw4RegistryContract) {
        let cw4_group_id = router.store_code(contract_cw4_group());

        let msg1 = cw4_group::msg::InstantiateMsg {
            admin: Some(ADMIN_ADDR.into()),
            members: vec![
                Member {
                    addr: ADDR1.into(),
                    weight: 11,
                },
                Member {
                    addr: ADDR2.into(),
                    weight: 6,
                },
                Member {
                    addr: ADDR3.into(),
                    weight: 11,
                },
            ],
        };

        // instantiate group
        let group1_addr = router
            .instantiate_contract(
                cw4_group_id,
                Addr::unchecked(ADDR2),
                &msg1,
                &[],
                "Consortium",
                None,
            )
            .unwrap();
        let group1_contract = Cw4GroupContract::new(group1_addr.clone());

        let msg2 = cw4_group::msg::InstantiateMsg {
            admin: Some(ADMIN_ADDR.into()),
            members: vec![
                Member {
                    addr: ADDR6.into(),
                    weight: 12,
                },
                Member {
                    addr: ADDR7.into(),
                    weight: 2,
                },
            ],
        };

        // instantiate group
        let group2_addr = router
            .instantiate_contract(
                cw4_group_id,
                Addr::unchecked(ADDR3),
                &msg2,
                &[],
                "Consortium2",
                None,
            )
            .unwrap();
        let group2_contract = Cw4GroupContract::new(group2_addr.clone());

        // instantiate cw4 registry
        let cw4_registry_code_id = router.store_code(contract_cw4_registry());
        let instantiate_msg = InstantiateMsg {};
        let registry_addr = router
            .instantiate_contract(
                cw4_registry_code_id,
                group1_addr.clone(),
                &instantiate_msg,
                &[],
                "Registry",
                None,
            )
            .unwrap();
        let registry_contract = Cw4RegistryContract::new(registry_addr.clone());

        // add hooks
        router
            .execute(
                Addr::unchecked(ADMIN_ADDR),
                group1_contract.add_hook(registry_contract.addr()).unwrap(),
            )
            .unwrap();
        router
            .execute(
                Addr::unchecked(ADMIN_ADDR),
                group2_contract.add_hook(registry_contract.addr()).unwrap(),
            )
            .unwrap();

        // register multisigs to registry
        let register_msg = ExecuteMsg::Register {
            group_addrs: vec![group1_addr.into_string(), group2_addr.into_string()],
        };
        let wasm_msg = WasmMsg::Execute {
            contract_addr: registry_addr.to_string(),
            msg: to_binary(&register_msg).unwrap(),
            funds: vec![],
        };
        router
            .execute(Addr::unchecked(ADDR2), wasm_msg.into())
            .unwrap();

        ((group1_contract, group2_contract), registry_contract)
    }

    #[test]
    fn test_update_members() {
        let mut router = mock_app();

        let ((group1_contract, _), registry_contract) = setup_environment(&mut router);

        let add = vec![
            Member {
                addr: ADDR6.to_string(),
                weight: 2,
            },
            Member {
                addr: ADDR7.to_string(),
                weight: 9,
            },
        ];
        let remove = vec![ADDR1.to_string(), ADDR2.to_string()];
        let update_msg = group1_contract.update_members(remove, add).unwrap();
        // current list: ADDR3, ADDR6, ADDR7

        router
            .execute(Addr::unchecked(ADMIN_ADDR), update_msg)
            .unwrap();

        // check if deleted
        let res = registry_contract.list_group(&router, ADDR1).unwrap();
        assert!(res.groups.is_empty());
    }

    #[test]
    fn test_membership_auth() {
        let mut router = mock_app();

        let ((group_contract, _), _) = setup_environment(&mut router);

        let hacker = Addr::unchecked("hacker");

        let op_add = vec![Member {
            addr: ADDR6.to_string(),
            weight: 2,
        }];
        let op_remove = vec![ADDR2.to_string()];
        let update_msg = group_contract.update_members(op_remove, op_add).unwrap();

        // only group can change
        let err = router.execute(hacker, update_msg).unwrap_err();
        let _expected = Error::new(ContractError::Unauthorized {});
        assert_matches!(err, _expected);
    }

    #[test]
    fn test_query_list_group() {
        let mut router = mock_app();

        let ((group_contract, _), registry_contract) = setup_environment(&mut router);

        let groups = registry_contract.list_group(&router, ADDR1).unwrap();
        assert_eq!(groups.groups, vec![group_contract.addr()])
    }
}
