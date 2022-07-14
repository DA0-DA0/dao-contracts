#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use osmo_bindings::{OsmosisMsg, OsmosisQuery};
use osmo_bindings_test::OsmosisModule;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, IsFrozenResponse, QueryMsg, SudoMsg};
use crate::state::{Config, BLACKLISTED_ADDRESSES, CONFIG, FREEZER_ALLOWANCES, MINTER_ALLOWANCES};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-usdc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO trigger CreateDenom msg
    OsmosisMsg::CreateDenom {
        subdenom: msg.subdenom,
    };

    let config = Config {
        owner: info.sender.clone(),
        is_frozen: false,
        denom: String::from("TODO"), // TODO: use denom from actual message
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint { to_address, amount } => {
            execute_mint(deps, env, info, to_address, amount)
        }
        ExecuteMsg::ChangeTokenFactoryAdmin { new_admin } => todo!(),
        ExecuteMsg::ChangeContractOwner { new_owner } => todo!(),
        ExecuteMsg::SetMinter { address, allowance } => todo!(),
        ExecuteMsg::SetBurner { address, allowance } => todo!(),
        ExecuteMsg::SetBlacklister { address, status } => todo!(),
        ExecuteMsg::SetFreezer { address, status } => {
            execute_set_freezer(deps, env, info, address, status)
        }
        ExecuteMsg::Burn { amount } => todo!(),
        ExecuteMsg::Blacklist { address, status } => {
            execute_blacklist(deps, env, info, address, status)
        }
        ExecuteMsg::Freeze { status } => execute_freeze(deps, env, info, status),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_address: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
    //     state.count += 1;
    //     Ok(state)
    // })?;

    deps.api.addr_validate(&to_address)?;
    let denom = query_denom(deps.as_ref())?;

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(ContractError::ZeroAmount {});
    }

    let allowance = MINTER_ALLOWANCES.update(
        deps.storage,
        info.sender,
        |allowance| -> StdResult<Uint128> {
            Ok(allowance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    // TODO execute actual MintMsg
    let mint_tokens_msg =
        OsmosisMsg::mint_contract_tokens(denom, amount, env.contract.address.into_string());

    let res = Response::new()
        .add_attribute("method", "mint_tokens")
        .add_message(mint_tokens_msg);

    Ok(Response::new().add_attribute("method", "try_increment"))
}

fn execute_set_freezer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response, ContractError> {
    // Check if sender is authorised to set freezer
    let config = CONFIG.load(deps.storage).unwrap();
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    FREEZER_ALLOWANCES.update(
        deps.storage,
        deps.api.addr_validate(address.as_str())?,
        |mut stat| -> Result<_, ContractError> {
            if let Some(current_status) = stat {
                if current_status == status {
                    return Err(ContractError::FreezerStatusUnchangedError { status });
                }
            }
            stat = Some(status);
            Ok(status)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "set_freezer")
        .add_attribute("freezer", address))
}

fn execute_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    status: bool,
) -> Result<Response, ContractError> {
    // check if the sender is allowed to freeze
    if let Some(freezer_status) = FREEZER_ALLOWANCES.may_load(deps.storage, info.sender)? {
        if freezer_status == false {
            return Err(ContractError::Unauthorized {});
        }

        // check if the status of the contract is already the same as the update
        let config = CONFIG.load(deps.storage)?;
        if config.is_frozen == status {
            return Err(ContractError::ContractFrozenStatusUnchangedError { status: status });
        } else {
            CONFIG.update(
                deps.storage,
                |mut config: Config| -> Result<_, ContractError> {
                    config.is_frozen = status;
                    Ok(config)
                },
            )?;

            Ok(Response::new().add_attribute("method", "execute_freeze"))
        }
    } else {
        return Err(ContractError::Unauthorized {});
    }
}

fn execute_blacklist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response, ContractError> {
    // TODO: check if sender is authorized

    // update blacklisted status
    BLACKLISTED_ADDRESSES.update(
        deps.storage,
        deps.api.addr_validate(address.as_str())?,
        |mut stat| -> Result<_, ContractError> {
            stat = Some(status);
            Ok(status)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "blacklist")
        .add_attribute("address", address)
        .add_attribute("new_value", status.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BeforeSend { from, to, amount } => beforesend_hook(deps, from, to, amount),
    }
}

pub fn beforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    amount: Vec<Coin>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.is_frozen {
        // is it neccesary to check each coin? or just always return error
        for coin in amount {
            if coin.denom == config.denom {
                return Err(ContractError::ContractFrozenError {
                    denom: config.denom,
                });
            }
        }
    }

    // Check if 'from' address is blacklisted
    let from_address = deps.api.addr_validate(from.as_str())?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, from_address)? {
        if is_blacklisted {
            return Err(ContractError::BlacklistedError { address: from });
        }
    };

    // Check if 'to' address is blacklisted
    let to_address = deps.api.addr_validate(to.as_str())?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, to_address)? {
        if is_blacklisted {
            return Err(ContractError::BlacklistedError { address: to });
        }
    };

    Ok(Response::new().add_attribute("method", "try_increment"))
}

// pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         state.count += 1;
//         Ok(state)
//     })?;

//     Ok(Response::new().add_attribute("method", "try_increment"))
// }

// pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         if info.sender != state.owner {
//             return Err(ContractError::Unauthorized {});
//         }
//         state.count = count;
//         Ok(state)
//     })?;
//     Ok(Response::new().add_attribute("method", "reset"))
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsFrozen {} => to_binary(&query_is_frozen(deps)?),
        QueryMsg::Denom {} => to_binary(&query_denom(deps)?),
    }
}

pub fn query_denom(deps: Deps) -> StdResult<String> {
    let config = CONFIG.load(deps.storage)?;
    return Ok(config.denom);
}

pub fn query_is_frozen(deps: Deps) -> StdResult<IsFrozenResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(IsFrozenResponse {
        is_frozen: config.is_frozen,
    })
}

#[cfg(test)]
mod tests {

    use crate::msg::DenomResponse;
    use crate::state::BLACKLISTER_ALLOWANCES;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Denom {}).unwrap();
        let value: DenomResponse = from_binary(&res).unwrap();
        assert_eq!("uusdc", value.denom);
    }

    #[test]
    fn freeze_contract() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // tests if the contract throws the right error for non-freezers
        let unauthorized_info = mock_info("anyone", &coins(1000, "earth"));
        let freeze_msg = ExecuteMsg::Freeze { status: true };
        let res = execute(deps.as_mut(), mock_env(), unauthorized_info, freeze_msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // Test if the contract is unfrozen
        let query_msg = QueryMsg::IsFrozen {};
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();

        let value: IsFrozenResponse = from_binary(&res).unwrap();
        assert_eq!(value.is_frozen, false);

        //  test if the contract throws the right error for freezer, but unauthorized
        add_freezer(deps.as_mut(), "false_freezer".to_string(), false);
        let info = mock_info("false_freezer", &coins(1000, "uusdc"));
        let freeze_msg = ExecuteMsg::Freeze { status: true };
        let err = execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!(
                "False freezer should generate a unauthorized error, but got {}",
                err
            ),
        }

        // test if the contract allows a authorized freezer to freeze the contract
        add_freezer(deps.as_mut(), "true_freezer".to_string(), true);
        let info = mock_info("true_freezer", &coins(1000, "uusdc"));
        let freeze_msg = ExecuteMsg::Freeze { status: true };
        let _res = execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap();

        // test if the contract allows a authorized freezer to unfreeze the contract
        add_freezer(deps.as_mut(), "true_freezer".to_string(), true);
        let info = mock_info("true_freezer", &coins(1000, "uusdc"));
        let freeze_msg = ExecuteMsg::Freeze { status: false };
        let _res = execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap();

        // test if the contract throws the right error for unchanged
        set_contract_config(deps.as_mut(), true);
        let info = mock_info("true_freezer", &coins(1000, "uusdc"));
        let freeze_msg = ExecuteMsg::Freeze { status: true };
        let err = execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap_err();
        match err {
            ContractError::ContractFrozenStatusUnchangedError { .. } => {}
            _ => panic!(
                "non-changing freeze msg should return FrozenStatusUnchangedError, but returns {}",
                err
            ),
        }
    }

    // test helper func
    #[allow(unused_assignments)]
    fn add_freezer(deps: DepsMut, address: String, status: bool) {
        FREEZER_ALLOWANCES
            .update(
                deps.storage,
                deps.api.addr_validate(&address.to_string()).unwrap(),
                |mut current_status| -> Result<_, ContractError> {
                    current_status = Some(status);

                    return Ok(status);
                },
            )
            .unwrap();
    }

    #[test]
    fn frozen_contract() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test unfrozen contract
        let sudo_msg = SudoMsg::BeforeSend {
            from: "from_address".to_string(),
            to: "to_address".to_string(),
            amount: coins(1000, "uusdc"),
        };
        let _res = sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();

        // Test frozen contract
        set_contract_config(deps.as_mut(), true);

        // Test if contract is frozen, Sudo msg with frozen coins will be blocked
        let sudo_msg = SudoMsg::BeforeSend {
            from: "from_address".to_string(),
            to: "to_address".to_string(),
            amount: coins(1000, "TODO"),
        };
        let res = sudo(deps.as_mut(), mock_env(), sudo_msg);
        let err = res.unwrap_err();
        match err {
            ContractError::ContractFrozenError { .. } => {}
            _ => {
                panic!("contract should be frozen, but is {}", err)
            }
        }

        // Test if contract is frozen, Sudo msg with non-frozen coins will not be blocked
        let sudo_msg = SudoMsg::BeforeSend {
            from: "from_address".to_string(),
            to: "to_address".to_string(),
            amount: coins(1000, "non-frozen"),
        };
        let _res = sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();
    }

    // test helper
    fn set_contract_config(deps: DepsMut, is_frozen: bool) {
        CONFIG
            .update(
                deps.storage,
                |mut config: Config| -> Result<_, ContractError> {
                    config.is_frozen = is_frozen;
                    Ok(config)
                },
            )
            .unwrap();
    }

    #[test]
    fn set_freezer() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            subdenom: String::from("udoge"),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // test setting freezers. set 2 because why not
        let set_freezer_msg = ExecuteMsg::SetFreezer {
            address: "freezer1".to_string(),
            status: true,
        };
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), set_freezer_msg).unwrap();
        let set_freezer_msg = ExecuteMsg::SetFreezer {
            address: "freezer2".to_string(),
            status: true,
        };
        let _res = execute(deps.as_mut(), mock_env(), info, set_freezer_msg).unwrap();

        // test if freezer1 can freeze
        let freeze_msg = ExecuteMsg::Freeze { status: true };
        let info = mock_info("freezer1", &coins(1000, "udoge"));
        let _res = execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap();

        // test if freezer can be unset
        let info = mock_info("creator", &coins(1000, "earth"));
        let set_freezer_msg = ExecuteMsg::SetFreezer {
            address: "freezer1".to_string(),
            status: false,
        };
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), set_freezer_msg).unwrap();

        let freeze_msg = ExecuteMsg::Freeze { status: false };
        let info = mock_info("freezer1", &coins(1000, "udoge"));
        let err = execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("should throw Unauthorized error but throws {}", err),
        }

        let info = mock_info("anyone", &coins(1000, "udoge"));
        let set_freezer_msg = ExecuteMsg::SetFreezer {
            address: "freezer2".to_string(),
            status: false,
        };
        let err = execute(deps.as_mut(), mock_env(), info, set_freezer_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("should throw Unauthorized error but throws {}", err),
        }
    }

    #[test]
    fn beforesend() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        add_to_blacklist(deps.as_mut(), "blacklisted".to_string(), true);

        // test when sender is blacklisted
        let sudo_msg = SudoMsg::BeforeSend {
            from: "blacklisted".to_string(),
            to: "to_address".to_string(),
            amount: coins(1000, "TODO"),
        };
        let err = sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap_err();
        match err {
            ContractError::BlacklistedError { .. } => {}
            _ => panic!(
                "Blacklisted sender should generate blacklistedError, not {}",
                err
            ),
        }

        // test when receiver is blacklisted
        let sudo_msg = SudoMsg::BeforeSend {
            from: "blacklisted".to_string(),
            to: "to_address".to_string(),
            amount: coins(1000, "TODO"),
        };
        let err = sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap_err();
        match err {
            ContractError::BlacklistedError { .. } => {}
            _ => panic!(
                "Blacklisted receiver should generate blacklistedError, not {}",
                err
            ),
        }

        // TODO: test when sender and receiver are not on the blacklist
        // TODO: test when the contract is frozen
        // TODO: test when the contract is frozen and the sender is blacklisted
    }

    // test helper
    #[allow(unused_assignments)]
    fn set_blacklister(deps: DepsMut, address: String, status: bool) {
        BLACKLISTER_ALLOWANCES
            .update(
                deps.storage,
                deps.api.addr_validate(&address.to_string()).unwrap(),
                |mut current_status| -> Result<_, ContractError> {
                    current_status = Some(status);
                    return Ok(status);
                },
            )
            .unwrap();
    }

    // test helper
    #[allow(unused_assignments)]
    fn add_to_blacklist(deps: DepsMut, address: String, status: bool) {
        BLACKLISTED_ADDRESSES
            .update(
                deps.storage,
                deps.api.addr_validate(&address.to_string()).unwrap(),
                |mut current_status| -> Result<_, ContractError> {
                    current_status = Some(status);

                    return Ok(status);
                },
            )
            .unwrap();
    }
}
