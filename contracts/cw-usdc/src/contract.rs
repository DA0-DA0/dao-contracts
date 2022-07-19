

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, 
};
use cw2::set_contract_version;

use cw_storage_plus::Map;
use osmo_bindings::{OsmosisMsg, OsmosisQuery };

// use osmo_bindings_test::OsmosisModule;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, IsFrozenResponse, QueryMsg, SudoMsg, DenomResponse};
use crate::state::{
    Config, BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES, BURNER_ALLOWANCES, CONFIG,
    FREEZER_ALLOWANCES, MINTER_ALLOWANCES,
};
use crate::helpers::build_denom;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-usdc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CREATE_DENOM_COST: Uint128 = Uint128::new(1000u128); // TEMPORARILY for the lack of a better solution (gRPC queries)

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // check info.funds
    check_funds("uosmo".to_string(), &info.funds, CREATE_DENOM_COST)?;
    
    // create full denom from contract addres using OsmosisModule.full_denom copy
    let contract_address = env.contract.address;
    let full_denom = build_denom(&contract_address, &msg.subdenom)?;

    let config = Config {
        owner: info.sender.clone(),
        is_frozen: false,
        denom: full_denom.clone(), 
    };
    
    CONFIG.save(deps.storage, &config)?;

    let create_denom_msg = OsmosisMsg::CreateDenom {
        subdenom: full_denom,
    };

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("contract", contract_address.to_string())
        .add_message(create_denom_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    match msg {
        ExecuteMsg::Mint { to_address, amount } => {
            execute_mint(deps, env, info, to_address, amount)
        }
        ExecuteMsg::Burn { amount } => execute_burn(deps, env, info, amount),
        ExecuteMsg::ChangeTokenFactoryAdmin { new_admin } => todo!(),
        ExecuteMsg::ChangeContractOwner { new_owner } => {
            execute_change_contract_owner(deps, env, info, new_owner)
        }
        ExecuteMsg::SetMinter { address, allowance } => {
            execute_set_minter(deps, env, info, address, allowance)
        }
        ExecuteMsg::SetBurner { address, allowance } => {
            execute_set_burner(deps, env, info, address, allowance)
        }
        ExecuteMsg::SetBlacklister { address, status } => {
            execute_set_blacklister(deps, env, info, address, status)
        }
        ExecuteMsg::SetFreezer { address, status } => {
            execute_set_freezer(deps, env, info, address, status)
        }
        ExecuteMsg::Blacklist { address, status } => {
            execute_blacklist(deps, env, info, address, status)
        }
        ExecuteMsg::Freeze { status } => execute_freeze(deps, env, info, status),
    }
}

fn check_funds(denom: String, funds: &Vec<Coin>, amount: Uint128) -> Result<(), ContractError>{
    // TODO: Do we want to check for too much funds here? Otherwise all the excess funds will remain locked in the contract 4evers

    if let Some(osmo) = funds.iter().find(|c| c.denom == denom) {
        if osmo.amount < amount {
            return Err(ContractError::NotEnoughFunds {denom, funds: osmo.amount.u128(), needed: amount.u128() })
        } else {
            return Ok(());
        }
    } else {
        return Err(ContractError::NotEnoughFunds {denom, funds: 0u128, needed: CREATE_DENOM_COST.u128()  })    
    }

}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_address: String,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {

    deps.api.addr_validate(&to_address)?;
    let denom = CONFIG.load(deps.storage).unwrap().denom;

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(ContractError::ZeroAmount {});
    }

    let _allowance = MINTER_ALLOWANCES.update(
        deps.storage,
        info.sender,
        |allowance| -> StdResult<Uint128> {
            Ok(allowance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    let mint_tokens_msg =
        OsmosisMsg::mint_contract_tokens(denom, amount, env.contract.address.into_string());

    // TODO: Second msg that sends tokens to the to_address

    Ok(
        Response::new()
            .add_attribute("method", "mint_tokens")
            .add_message(mint_tokens_msg)
    )
}

fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let denom = CONFIG.load(deps.storage).unwrap().denom;

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(ContractError::ZeroAmount {});
    }

    // Contract needs to own the coins it wants to burn 
    check_funds(denom.clone(), &info.funds, amount)?;



    let _allowance = BURNER_ALLOWANCES.update(
        deps.storage,
        info.sender.clone(),
        |allowance| -> StdResult<Uint128> {
            Ok(allowance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    // TODO execute actual BurnMsg -> needs to be the contract address or maybe need to include in info.funds... see whats possible
    // burns tokens that are owned by this contract.
    let burn_tokens_msg = 
        OsmosisMsg::burn_contract_tokens(denom, amount, "".to_string());


    Ok(
        Response::new()
            .add_attribute("method", "execute_burn")
            .add_attribute("amount", amount.to_string())
            .add_message(burn_tokens_msg)
    )
}

fn execute_change_contract_owner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // TODO: check using the comment below and save state instead of update
    // check_contract_owner(deps.as_ref(), info.sender)?; 
    let val_address = deps.api.addr_validate(address.as_str())?;

    CONFIG.update(
        deps.storage,
        |mut config: Config| -> Result<Config, ContractError> {
            if config.owner == info.sender {
                config.owner = val_address;
                return Ok(config);
            }

            return Err(ContractError::Unauthorized {});
        },
    )?;

    Ok(Response::new().add_attribute("method", "change_contract_owner"))
}

fn execute_set_blacklister(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_contract_owner(deps.as_ref(), info.sender)?;

    set_bool_allowance(deps, &address, BLACKLISTER_ALLOWANCES, status)?;

    Ok(Response::new()
        .add_attribute("method", "set_blacklister")
        .add_attribute("blacklister", address)
        .add_attribute("status", status.to_string()))
}

fn execute_set_freezer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // Check if sender is authorised to set freezer
    check_contract_owner(deps.as_ref(), info.sender)?;

    set_bool_allowance(deps, &address, FREEZER_ALLOWANCES, status)?;

    Ok(Response::new()
        .add_attribute("method", "set_freezer")
        .add_attribute("freezer", address)
        .add_attribute("status", status.to_string()))
}

fn set_bool_allowance(
    deps: DepsMut,
    address: &String,
    allowances: Map<Addr, bool>,
    status: bool,
) -> Result<bool, ContractError> {
    return allowances.update(
        deps.storage,
        deps.api.addr_validate(address.as_str())?,
        |mut stat| -> Result<_, ContractError> {
            if let Some(current_status) = stat {
                if current_status == status {
                    return Err(ContractError::FreezerStatusUnchanged { status });
                }
            }
            stat = Some(status);
            Ok(status)
        },
    );
}

fn check_contract_owner(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage).unwrap();
    if config.owner != sender {
        return Err(ContractError::Unauthorized {});
    } else {
        Ok(())
    }
}

fn set_int_allowance(
    deps: DepsMut,
    allowances: Map<Addr, Uint128>,
    address: &String,
    amount: Uint128,
) -> Result<(), ContractError> {
    // TODO What if the allowance doesnt change, like i check for at bool_allowance
    let res = allowances.save(
        deps.storage,
        deps.api.addr_validate(address.as_str())?,
            &amount);
    match res {
        Ok(()) => Ok(()),
        Err(error) => Err(ContractError::Std(error))
    }
}

fn execute_set_burner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_contract_owner(deps.as_ref(), info.sender)?;

    // Set Burner allowance
    set_int_allowance(deps, BURNER_ALLOWANCES, &address, amount)?;

    Ok(Response::new()
        .add_attribute("method", "set_burner")
        .add_attribute("burner", address)
        .add_attribute("amount", amount))
}

fn execute_set_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    amount: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_contract_owner(deps.as_ref(), info.sender)?;

    // Set minter allowance
    set_int_allowance(deps, MINTER_ALLOWANCES, &address, amount)?;

    Ok(Response::new()
        .add_attribute("method", "set_minter")
        .add_attribute("minter", address)
        .add_attribute("amount", amount))
}

fn execute_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    // check if the sender is allowed to freeze
    check_allowance(&deps.as_ref(), info.clone(), FREEZER_ALLOWANCES)?;

    let config = CONFIG.load(deps.storage)?;
    if config.is_frozen == status {
        return Err(ContractError::ContractFrozenStatusUnchanged { status });
    } else {
        CONFIG.update(
            deps.storage,
            |mut config: Config| -> Result<_, ContractError> {
                config.is_frozen = status;
                Ok(config)
            },
        )?;

        Ok(Response::new()
            .add_attribute("method", "execute_freeze")
            .add_attribute("status", status.to_string()))
    }
}

fn execute_blacklist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    status: bool,
) -> Result<Response<OsmosisMsg>, ContractError> {
    check_allowance(&deps.as_ref(), info, BLACKLISTER_ALLOWANCES)?;

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
        .add_attribute("status", status.to_string()))
}

fn check_allowance(
    deps: &Deps,
    info: MessageInfo,
    allowances: Map<Addr, bool>,
) -> Result<(), ContractError> {
    let res = allowances.load(deps.storage, info.sender);
    match res {
        Ok(authorized) => {
            if !authorized {
                return Err(ContractError::Unauthorized {});
            }
        }
        Err(error) => {
            if let cosmwasm_std::StdError::NotFound { .. } = error {
                return Err(ContractError::Unauthorized {});
            } else {
                return Err(ContractError::Std(error));
            }
        }
    }
    Ok(())
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
        for coin in amount.clone() {
            if coin.denom == config.denom {
                return Err(ContractError::ContractFrozen {
                    denom: config.denom,
                });
            }
        }
    }

    // Check if 'from' address is blacklisted
    let from_address = deps.api.addr_validate(from.as_str())?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, from_address)? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address: from });
        }
    };

    // Check if 'to' address is blacklisted
    let to_address = deps.api.addr_validate(to.as_str())?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, to_address.clone())? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address: to });
        }
    };

    Ok(Response::new()
        .add_attribute("method", "before_send")
    )
    
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsFrozen {} => to_binary(&query_is_frozen(deps)?),
        QueryMsg::Denom {} => to_binary(&query_denom(deps)?),
    }
}

pub fn query_denom(deps: Deps) -> StdResult<DenomResponse> {
    let config = CONFIG.load(deps.storage)?;
    return Ok(DenomResponse {
        denom: config.denom
    });
//    return Ok(config.denom);
}

pub fn query_is_frozen(deps: Deps) -> StdResult<IsFrozenResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(IsFrozenResponse {
        is_frozen: config.is_frozen,
    })
}


// TODO: QUERIES 
// owner
// allowances
// blacklisted
// see https://github.com/mars-protocol/fields-of-mars/blob/v1.0.0/packages/fields-of-mars/src/martian_field.rs#L465-L473 


#[cfg(test)]
mod tests {
    use crate::msg::DenomResponse;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len()); 

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Denom {}).unwrap();
        let value: DenomResponse = from_binary(&res).unwrap();
        assert_eq!("factory/cosmos2contract/uusdc", value.denom);
    }

    #[test]
    fn change_contract_owner() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let new_info = mock_info("new_owner", &coins(1000, "ueeur"));
        let change_msg = ExecuteMsg::ChangeContractOwner {
            new_owner: new_info.sender.clone().into_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), change_msg.clone()).unwrap();

        let res = check_contract_owner(deps.as_ref(), new_info.sender.clone()).unwrap();

        // test for if non owner(previous owner) tries to change owner

        let change_msg = ExecuteMsg::ChangeContractOwner {
            new_owner: new_info.sender.clone().into_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), change_msg.clone()).unwrap_err();
        match err {
            ContractError::Unauthorized {  } => (),
            error => panic!("should generate Unauthorised but returns {}", error)
        }

        
    }

    #[test]
    fn freeze_contract() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uusdc"),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));

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
            ContractError::ContractFrozenStatusUnchanged { .. } => {}
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
        let info = mock_info("creator", &coins(1000, "uosmo"));
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
            amount: coins(1000, "uusdc"),
        };
        let res = sudo(deps.as_mut(), mock_env(), sudo_msg);
        let err = res.unwrap_err();
        match err {
            ContractError::ContractFrozen { .. } => {}
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
        let info = mock_info("creator", &coins(1000, "uosmo"));
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
    fn set_blacklister() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            subdenom: String::from("udoge"),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // test setting freezers. set 2 because why not
        let set_blacklister_msg = ExecuteMsg::SetBlacklister {
            address: "blacklister1".to_string(),
            status: true,
        };
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), set_blacklister_msg).unwrap();

        let set_blacklister_msg = ExecuteMsg::SetBlacklister {
            address: "blacklister2".to_string(),
            status: true,
        };
        let _res = execute(deps.as_mut(), mock_env(), info, set_blacklister_msg).unwrap();

        // test if blacklister1 can freeze
        let blacklist_msg = ExecuteMsg::Blacklist {
            address: "someone".to_string(),
            status: true,
        };
        let info = mock_info("blacklister1", &coins(1000, "udoge"));
        let _res = execute(deps.as_mut(), mock_env(), info, blacklist_msg).unwrap();

        // test if freezer can be unset
        let info = mock_info("creator", &coins(1000, "earth"));
        let set_blacklister_msg = ExecuteMsg::SetBlacklister {
            address: "blacklister1".to_string(),
            status: false,
        };
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), set_blacklister_msg).unwrap();

        let blacklist_msg = ExecuteMsg::Blacklist {
            address: "anyone2".to_string(),
            status: false,
        };
        let info = mock_info("blacklister1", &coins(1000, "udoge"));
        let err = execute(deps.as_mut(), mock_env(), info, blacklist_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("should throw Unauthorized error but throws {}", err),
        }

        let info = mock_info("anyone", &coins(1000, "udoge"));
        let set_blacklister_msg = ExecuteMsg::SetBlacklister {
            address: "blacklister2".to_string(),
            status: false,
        };
        let err = execute(deps.as_mut(), mock_env(), info, set_blacklister_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("should throw Unauthorized error but throws {}", err),
        }
    }

    #[test]

    fn set_burner() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            subdenom: "uakt".to_string(),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let burner_info = mock_info("burner", &coins(1000, "uakt"));
        let set_burner_msg = ExecuteMsg::SetBurner {
            address: burner_info.sender.to_string(),
            allowance: Uint128::from(1000u64),
        };

        let res = execute(deps.as_mut(), mock_env(), info.clone(), set_burner_msg).unwrap();

        let burn_msg = ExecuteMsg::Burn {
            amount: Uint128::from(100u64),
        };
        let res = execute(deps.as_mut(), mock_env(), burner_info.clone(), burn_msg).unwrap();

        // mint more then allowance
        let burn_msg = ExecuteMsg::Burn {
            amount: Uint128::from(950u64),
        };
        let err = execute(deps.as_mut(), mock_env(), burner_info.clone(), burn_msg).unwrap_err();
        // TODO: match error
    }
    #[test]
    fn set_minter() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            subdenom: "uakt".to_string(),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let minter_info = mock_info("minter", &coins(1000, "uakt"));
        let set_mint_msg = ExecuteMsg::SetMinter {
            address: minter_info.sender.to_string(),
            allowance: Uint128::from(1000u64),
        };

        let res = execute(deps.as_mut(), mock_env(), info.clone(), set_mint_msg).unwrap();

        let mint_msg = ExecuteMsg::Mint {
            to_address: minter_info.sender.to_string(),
            amount: Uint128::from(100u64),
        };
        let res = execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg).unwrap();

        // mint more then allowance
        let mint_msg = ExecuteMsg::Mint {
            to_address: minter_info.sender.to_string(),
            amount: Uint128::from(950u64),
        };
        let err = execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg).unwrap_err();
        // TODO: match error
    }

    #[test]
    fn beforesend() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            subdenom: String::from("uquarks"),
        };
        let info = mock_info("creator", &coins(1000, "uosmo"));
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
            ContractError::Blacklisted { .. } => {}
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
            ContractError::Blacklisted { .. } => {}
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
    fn add_blacklister(deps: DepsMut, address: String, status: bool) {
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
