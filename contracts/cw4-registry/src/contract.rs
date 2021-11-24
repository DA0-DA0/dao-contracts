#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::INDEX;
use cw2::set_contract_version;
use cw4_group::helpers::Cw4GroupContract;
use cw4::MemberChangedHookMsg;

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
        ExecuteMsg::Register { contract_addr } => execute_register(deps, env, info, contract_addr),
        ExecuteMsg::MemberChangedHook(msg) => execute_member_changed_hook(deps, env, info, msg)
    }
}

pub fn execute_register(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    contract_addr: String,
) -> Result<Response, ContractError> {
    let group_addr = deps.api.addr_validate(&contract_addr)?;

    let contract = Cw4GroupContract::new(group_addr.clone());
    let members = contract.list_members(&deps.querier, None, None)?;

    for m in members {
        let member_addr = deps.api.addr_validate(m.addr.as_str())?;
        INDEX.save(deps.storage, (&member_addr, &group_addr), &Empty {})?;
    }

    Ok(Response::default())
}

pub fn execute_member_changed_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    for md in msg.diffs {
        // add new addresses
        if md.new.is_some() {
            let addr = deps.api.addr_validate(md.key.as_str())?;
            INDEX.save(deps.storage, (&info.sender, &addr), &Empty{})?;
        }

        // remove old addresses
        if md.old.is_some() {
            let addr = deps.api.addr_validate(md.key.as_str())?;
            INDEX.remove(deps.storage, (&info.sender, &addr));
        }
    }

    Ok(Response::default())
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::{to_binary, Addr, Empty, WasmMsg};
    use cw4::Member;
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};
    use cw4_group::helpers::Cw4GroupContract;

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
    const ADDR4: &str = "add4";
    const ADDR5: &str = "add5";
    const ADDR6: &str = "add6";
    const ADDR7: &str = "add7";

    #[test]
    fn cw4_register_test() {
        let mut router = mock_app();

        let cw4_group_id = router.store_code(contract_cw4_group());

        let cw4_instantiate_msg = cw4_group::msg::InstantiateMsg {
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
        let group_addr = router
            .instantiate_contract(
                cw4_group_id,
                Addr::unchecked(ADDR2),
                &cw4_instantiate_msg,
                &[],
                "Consortium",
                None,
            )
            .unwrap();
        let group_contract = Cw4GroupContract::new(group_addr.clone());

        // instantiate cw4 registry
        let cw4_registry_code_id = router.store_code(contract_cw4_registry());
        let instantiate_msg = InstantiateMsg {};
        let cw4_registry_addr = router
            .instantiate_contract(
                cw4_registry_code_id,
                group_addr.clone(),
                &instantiate_msg,
                &[],
                "Registry",
                None,
            )
            .unwrap();

        // register multisig to registry
        let register_msg = ExecuteMsg::Register {
            contract_addr: group_addr.clone().into_string(),
        };
        let wasm_msg = WasmMsg::Execute {
            contract_addr: cw4_registry_addr.to_string(),
            msg: to_binary(&register_msg).unwrap(),
            funds: vec![],
        };

        router
            .execute(Addr::unchecked(ADDR2), wasm_msg.into())
            .unwrap();

        let add = vec![
            Member{ addr: ADDR6.to_string(), weight: 2 },
            Member{ addr: ADDR7.to_string(), weight: 9 }
        ];
        let remove = vec![ADDR1.to_string(), ADDR2.to_string()];
        let update_msg = group_contract.update_members(remove, add).unwrap();
        // current list: ADDR3, ADDR6, ADDR7

        router.execute(group_addr, update_msg).unwrap();
    }
}
