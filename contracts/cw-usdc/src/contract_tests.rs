#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Uint128};

// use osmo_bindings_test::OsmosisModule;

use crate::contract;
use crate::error::ContractError;
use crate::helpers::{build_denom, check_is_contract_owner};
use crate::msg::{DenomResponse, ExecuteMsg, InstantiateMsg, IsFrozenResponse, QueryMsg, SudoMsg};
use crate::state::{
    Config, BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES, CONFIG, FREEZER_ALLOWANCES,
};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, from_binary};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        subdenom: String::from("uusdc"),
    };
    let info = mock_info("creator", &coins(1000, "uosmo"));

    // instantiate with enough funds provided should succeed
    let res = contract::instantiate(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    assert_eq!(2, res.messages.len());

    // it worked, let's query the state
    let res = contract::query(deps.as_ref(), mock_env(), QueryMsg::Denom {}).unwrap();
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
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let new_info = mock_info("new_owner", &coins(1000, "ueeur"));
    let change_msg = ExecuteMsg::ChangeContractOwner {
        new_owner: new_info.sender.clone().into_string(),
    };

    contract::execute(deps.as_mut(), mock_env(), info.clone(), change_msg.clone()).unwrap();

    check_is_contract_owner(deps.as_ref(), new_info.sender.clone()).unwrap();

    // test for if non owner(previous owner) tries to change owner

    let change_msg = ExecuteMsg::ChangeContractOwner {
        new_owner: new_info.sender.clone().into_string(),
    };
    let err =
        contract::execute(deps.as_mut(), mock_env(), info.clone(), change_msg.clone()).unwrap_err();
    match err {
        ContractError::Unauthorized {} => (),
        error => panic!("should generate Unauthorised but returns {}", error),
    }
}

#[test]
fn freeze_contract() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        subdenom: String::from("uusdc"),
    };
    let info = mock_info("creator", &coins(10000000, "uosmo"));

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // tests if the contract throws the right error for non-freezers
    let unauthorized_info = mock_info("anyone", &[]);
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::Freeze { status: true },
    );
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // Test to make sure the contract is unfrozen
    let query_msg = QueryMsg::IsFrozen {};
    let res = contract::query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: IsFrozenResponse = from_binary(&res).unwrap();
    assert!(!value.is_frozen);

    //  test if the contract throws the right error for freezer, but unauthorized
    add_freezer(deps.as_mut(), "false_freezer".to_string(), false);
    let info = mock_info("false_freezer", &[]);
    let freeze_msg = ExecuteMsg::Freeze { status: true };
    let err = contract::execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap_err();
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
    let _res = contract::execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap();

    // test if the contract allows a authorized freezer to unfreeze the contract
    add_freezer(deps.as_mut(), "true_freezer".to_string(), true);
    let info = mock_info("true_freezer", &coins(1000, "uusdc"));
    let freeze_msg = ExecuteMsg::Freeze { status: false };
    let _res = contract::execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap();
}

// test helper func
#[allow(unused_assignments)]
fn add_freezer(deps: DepsMut, address: String, status: bool) {
    FREEZER_ALLOWANCES
        .update(
            deps.storage,
            &deps.api.addr_validate(&address).unwrap(),
            |mut _current_status| -> Result<_, ContractError> {
                _current_status = Some(status);

                Ok(status)
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
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Test unfrozen contract
    let sudo_msg = SudoMsg::BeforeSend {
        from: "from_address".to_string(),
        to: "to_address".to_string(),
        amount: coins(
            1000,
            build_denom(&&mock_env().contract.address, "uusdc").unwrap(),
        ),
    };
    let _res = contract::sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();

    // Test frozen contract
    set_contract_config(deps.as_mut(), true);
    let res: IsFrozenResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::IsFrozen {}).unwrap())
            .unwrap();
    assert!(res.is_frozen);

    // Test if contract is frozen, Sudo msg with frozen coins should be blocked
    let sudo_msg = SudoMsg::BeforeSend {
        from: "from_address".to_string(),
        to: "to_address".to_string(),
        amount: coins(
            1000,
            build_denom(&&mock_env().contract.address, "uusdc").unwrap(),
        ),
    };
    let res = contract::sudo(deps.as_mut(), mock_env(), sudo_msg);
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
    let _res = contract::sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();
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
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // test setting freezers. set 2 because why not
    let set_freezer_msg = ExecuteMsg::SetFreezer {
        address: "freezer1".to_string(),
        status: true,
    };
    let _res = contract::execute(deps.as_mut(), mock_env(), info.clone(), set_freezer_msg).unwrap();
    let set_freezer_msg = ExecuteMsg::SetFreezer {
        address: "freezer2".to_string(),
        status: true,
    };
    let _res = contract::execute(deps.as_mut(), mock_env(), info, set_freezer_msg).unwrap();

    // test if freezer1 can freeze
    let freeze_msg = ExecuteMsg::Freeze { status: true };
    let info = mock_info("freezer1", &coins(1000, "udoge"));
    let _res = contract::execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap();

    // test if freezer can be unset
    let info = mock_info("creator", &coins(1000, "earth"));
    let set_freezer_msg = ExecuteMsg::SetFreezer {
        address: "freezer1".to_string(),
        status: false,
    };
    let _res = contract::execute(deps.as_mut(), mock_env(), info.clone(), set_freezer_msg).unwrap();

    let freeze_msg = ExecuteMsg::Freeze { status: false };
    let info = mock_info("freezer1", &coins(1000, "udoge"));
    let err = contract::execute(deps.as_mut(), mock_env(), info, freeze_msg).unwrap_err();
    match err {
        ContractError::Unauthorized {} => {}
        _ => panic!("should throw Unauthorized error but throws {}", err),
    }

    let info = mock_info("anyone", &coins(1000, "udoge"));
    let set_freezer_msg = ExecuteMsg::SetFreezer {
        address: "freezer2".to_string(),
        status: false,
    };
    let err = contract::execute(deps.as_mut(), mock_env(), info, set_freezer_msg).unwrap_err();
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
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // test setting freezers. set 2 because why not
    let set_blacklister_msg = ExecuteMsg::SetBlacklister {
        address: "blacklister1".to_string(),
        status: true,
    };
    let _res =
        contract::execute(deps.as_mut(), mock_env(), info.clone(), set_blacklister_msg).unwrap();

    let set_blacklister_msg = ExecuteMsg::SetBlacklister {
        address: "blacklister2".to_string(),
        status: true,
    };
    let _res = contract::execute(deps.as_mut(), mock_env(), info, set_blacklister_msg).unwrap();

    // test if blacklister1 can freeze
    let blacklist_msg = ExecuteMsg::Blacklist {
        address: "someone".to_string(),
        status: true,
    };
    let info = mock_info("blacklister1", &coins(1000, "udoge"));
    let _res = contract::execute(deps.as_mut(), mock_env(), info, blacklist_msg).unwrap();

    // test if freezer can be unset
    let info = mock_info("creator", &coins(1000, "earth"));
    let set_blacklister_msg = ExecuteMsg::SetBlacklister {
        address: "blacklister1".to_string(),
        status: false,
    };
    let _res =
        contract::execute(deps.as_mut(), mock_env(), info.clone(), set_blacklister_msg).unwrap();

    let blacklist_msg = ExecuteMsg::Blacklist {
        address: "anyone2".to_string(),
        status: false,
    };
    let info = mock_info("blacklister1", &coins(1000, "udoge"));
    let err = contract::execute(deps.as_mut(), mock_env(), info, blacklist_msg).unwrap_err();
    match err {
        ContractError::Unauthorized {} => {}
        _ => panic!("should throw Unauthorized error but throws {}", err),
    }

    let info = mock_info("anyone", &coins(1000, "udoge"));
    let set_blacklister_msg = ExecuteMsg::SetBlacklister {
        address: "blacklister2".to_string(),
        status: false,
    };
    let err = contract::execute(deps.as_mut(), mock_env(), info, set_blacklister_msg).unwrap_err();
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
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let full_denom = build_denom(&mock_env().contract.address, msg.subdenom.as_str()).unwrap();
    let burner_info = mock_info("burner", &coins(1000, full_denom.as_str()));
    let set_burner_msg = ExecuteMsg::SetBurner {
        address: burner_info.sender.to_string(),
        allowance: Uint128::from(1000u64),
    };

    contract::execute(deps.as_mut(), mock_env(), info.clone(), set_burner_msg).unwrap();

    let burn_msg = ExecuteMsg::Burn {
        amount: Uint128::from(100u64),
    };
    contract::execute(deps.as_mut(), mock_env(), burner_info.clone(), burn_msg).unwrap();

    let burner_info = mock_info("burner", &coins(100, full_denom.as_str()));
    // mint more then allowance
    let burn_msg = ExecuteMsg::Burn {
        amount: Uint128::from(950u64),
    };
    let err =
        contract::execute(deps.as_mut(), mock_env(), burner_info.clone(), burn_msg).unwrap_err();
    // TODO: match error
}
#[test]
fn set_minter() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        subdenom: "uakt".to_string(),
    };
    let info = mock_info("creator", &coins(1000, "uosmo"));
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let minter_info = mock_info("minter", &coins(1000, "uakt"));
    let set_mint_msg = ExecuteMsg::SetMinter {
        address: minter_info.sender.to_string(),
        allowance: Uint128::from(1000u64),
    };

    let res = contract::execute(deps.as_mut(), mock_env(), info.clone(), set_mint_msg).unwrap();

    let mint_msg = ExecuteMsg::Mint {
        to_address: minter_info.sender.to_string(),
        amount: Uint128::from(100u64),
    };
    let res = contract::execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg).unwrap();

    // mint more then allowance
    let mint_msg = ExecuteMsg::Mint {
        to_address: minter_info.sender.to_string(),
        amount: Uint128::from(950u64),
    };
    let err =
        contract::execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg).unwrap_err();
    // TODO: match error
}

#[test]
fn beforesend() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        subdenom: String::from("uquarks"),
    };
    let info = mock_info("creator", &coins(1000, "uosmo"));
    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    add_to_blacklist(deps.as_mut(), "blacklisted".to_string(), true);
    // test when sender is blacklisted
    let sudo_msg = SudoMsg::BeforeSend {
        from: "blacklisted".to_string(),
        to: "to_address".to_string(),
        amount: coins(1000, "TODO"),
    };
    let err = contract::sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap_err();
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
    let err = contract::sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap_err();
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
            &deps.api.addr_validate(&address).unwrap(),
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
            &deps.api.addr_validate(&address).unwrap(),
            |mut _current_status| -> Result<_, ContractError> {
                _current_status = Some(status);

                Ok(status)
            },
        )
        .unwrap();
}
