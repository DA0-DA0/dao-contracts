#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::INDEX;
use cw2::set_contract_version;
use cw4_group::helpers::Cw4GroupContract;

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
    const ADDR2: &str = "somebody";
    const ADDR3: &str = "else";
    const ADDR4: &str = "funny";

    #[test]
    fn cw4_registers() {
        let mut router = mock_app();

        let cw4_group_id = router.store_code(contract_cw4_group());

        let cw4_instantiate_msg = cw4_group::msg::InstantiateMsg {
            admin: Some(ADMIN_ADDR.into()),
            members: vec![
                Member {
                    addr: ADDR2.into(),
                    weight: 11,
                },
                Member {
                    addr: ADDR3.into(),
                    weight: 6,
                },
            ],
        };

        // instantiate cw4 group
        let multisig_addr = router
            .instantiate_contract(
                cw4_group_id,
                Addr::unchecked(ADDR2),
                &cw4_instantiate_msg,
                &[],
                "Consortium",
                None,
            )
            .unwrap();

        // instantiate cw4 registry
        let cw4_registry_code_id = router.store_code(contract_cw4_registry());
        let instantiate_msg = InstantiateMsg {};
        let cw4_registry_addr = router
            .instantiate_contract(
                cw4_registry_code_id,
                multisig_addr.clone(),
                &instantiate_msg,
                &[],
                "Registry",
                None,
            )
            .unwrap();

        // register multisig to registry
        let register_msg = ExecuteMsg::Register {
            contract_addr: multisig_addr.into_string(),
        };
        let wasm_msg = WasmMsg::Execute {
            contract_addr: cw4_registry_addr.to_string(),
            msg: to_binary(&register_msg).unwrap(),
            funds: vec![],
        };

        router
            .execute(Addr::unchecked(ADDR2), wasm_msg.into())
            .unwrap();
    }
}
