#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Uint128};

// use osmo_bindings_test::OsmosisModule;

use crate::contract;
use crate::error::ContractError;
use crate::helpers::check_is_contract_owner;
use crate::msg::{
    AllowanceResponse, DenomResponse, ExecuteMsg, InstantiateMsg, IsFrozenResponse, OwnerResponse,
    QueryMsg, StatusResponse, SudoMsg,
};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, coins, from_binary, Addr};

static CREATOR_ADDRESS: &str = "creator";

// test helper
#[allow(unused_assignments)]
fn initialize_contract(deps: DepsMut) -> (Addr, String) {
    let denom = String::from("factory/creator/uusdc");
    let msg = InstantiateMsg {
        denom: denom.clone(),
    };
    let info = mock_info(CREATOR_ADDRESS, &[]);

    // instantiate with enough funds provided should succeed
    contract::instantiate(deps, mock_env(), info.clone(), msg.clone()).unwrap();

    (info.sender, denom)
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let (owner, denom) = initialize_contract(deps.as_mut());

    // it worked, let's query the state
    let res: DenomResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::Denom {}).unwrap())
            .unwrap();
    assert_eq!(denom, res.denom);

    let res: OwnerResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap())
            .unwrap();
    assert_eq!(owner, res.address);

    // Test to make sure the contract is unfrozen
    let res: IsFrozenResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::IsFrozen {}).unwrap())
            .unwrap();
    assert!(!res.is_frozen);
}

#[test]
fn change_contract_owner() {
    let mut deps = mock_dependencies();

    let (original_owner, _) = initialize_contract(deps.as_mut());

    let new_owner_addr = "new_owner";

    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(original_owner.as_str(), &[]),
        ExecuteMsg::ChangeContractOwner {
            new_owner: String::from(new_owner_addr),
        },
    )
    .unwrap();

    check_is_contract_owner(deps.as_ref(), Addr::unchecked(new_owner_addr)).unwrap();

    // test for error if non owner (previous owner) tries to change owner
    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(original_owner.as_str(), &[]),
        ExecuteMsg::ChangeContractOwner {
            new_owner: String::from(new_owner_addr),
        },
    )
    .unwrap_err();
    match err {
        ContractError::Unauthorized {} => (),
        error => panic!("should generate Unauthorised but returns {}", error),
    }
}

#[test]
fn change_tokenfactory_admin() {
    let mut deps = mock_dependencies();

    let (original_owner, _) = initialize_contract(deps.as_mut());

    let new_admin_addr = "new_admin";

    // don't allow anoyone other than contract admin to transfer tokenfactory adminship
    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info("anyone", &[]),
        ExecuteMsg::ChangeTokenFactoryAdmin {
            new_admin: String::from(new_admin_addr),
        },
    )
    .unwrap_err();
    match err {
        ContractError::Unauthorized {} => (),
        error => panic!("should generate Unauthorised but returns {}", error),
    }

    // allow current contract owner to change tokenfactory admin
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(original_owner.as_str(), &[]),
        ExecuteMsg::ChangeTokenFactoryAdmin {
            new_admin: String::from(new_admin_addr),
        },
    )
    .unwrap();
    assert!(res.messages.len() == 1);
}

#[test]
fn freezing() {
    let mut deps = mock_dependencies();

    let (_, denom) = initialize_contract(deps.as_mut());

    // tests if the contract throws the right error for non-existant freezers
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

    let new_freezer_address = "freezer";

    // test unfrozen contract allows sends
    let sudo_msg = SudoMsg::BeforeSend {
        from: "from_address".to_string(),
        to: "to_address".to_string(),
        amount: coins(1000, denom.clone()),
    };
    contract::sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();

    // admin adds freezer
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetFreezer {
            address: String::from(new_freezer_address),
            status: true,
        },
    )
    .unwrap();

    // query freezer status is true
    let res: StatusResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsFreezer {
                address: String::from(new_freezer_address),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.status);

    // new freezer can freeze contract
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(new_freezer_address, &[]),
        ExecuteMsg::Freeze { status: true },
    )
    .unwrap();

    // query contract frozen is true
    let res: IsFrozenResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::IsFrozen {}).unwrap())
            .unwrap();
    assert!(res.is_frozen);

    // test if contract is frozen, Sudo msg with frozen coins should be blocked
    let sudo_msg = SudoMsg::BeforeSend {
        from: "from_address".to_string(),
        to: "to_address".to_string(),
        amount: coins(1000, denom.clone()),
    };
    let res = contract::sudo(deps.as_mut(), mock_env(), sudo_msg);
    let err = res.unwrap_err();
    match err {
        ContractError::ContractFrozen { .. } => {}
        _ => {
            panic!("contract should be frozen, but is {}", err)
        }
    }

    // test if contract is frozen, Sudo msg with multiple denoms will be blocked
    let _res = contract::sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::BeforeSend {
            from: "from_address".to_string(),
            to: "to_address".to_string(),
            amount: vec![coin(1000, "somethingelse"), coin(1000, denom.clone())],
        },
    )
    .unwrap_err();

    // admin can remove freezing capabilitity
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetFreezer {
            address: String::from(new_freezer_address),
            status: false,
        },
    )
    .unwrap();

    // query freezer status is false
    let res: StatusResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsFreezer {
                address: String::from(new_freezer_address),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(!res.status);

    // make sure freezer can no longer unfreeze contract
    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(new_freezer_address, &[]),
        ExecuteMsg::Freeze { status: true },
    )
    .unwrap_err();
    match err {
        ContractError::Unauthorized {} => {}
        _ => panic!(
            "False freezer should generate a unauthorized error, but got {}",
            err
        ),
    }
}

#[test]
fn blacklists() {
    // initialize contracts
    let mut deps = mock_dependencies();
    let (owner, denom) = initialize_contract(deps.as_mut());
    let blacklister_address = "blacklister";
    let blacklistee_address = "blacklistee";

    // tests if the contract throws the right error for non-existant blacklister
    let unauthorized_info = mock_info("anyone", &[]);
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::Blacklist {
            address: String::from(blacklistee_address),
            status: true,
        },
    );
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // test can send from blacklistee address
    let sudo_msg = SudoMsg::BeforeSend {
        from: blacklistee_address.to_string(),
        to: blacklister_address.to_string(),
        amount: coins(1000, denom.clone()),
    };
    contract::sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();

    // admin adds blacklister
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetBlacklister {
            address: String::from(blacklister_address),
            status: true,
        },
    )
    .unwrap();

    // query blacklister status is true
    let res: StatusResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsBlacklister {
                address: String::from(blacklister_address),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.status);

    // new blacklister can blacklist blacklistee
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(blacklister_address, &[]),
        ExecuteMsg::Blacklist {
            address: String::from(blacklistee_address),
            status: true,
        },
    )
    .unwrap();

    // query blacklistee status is true
    let res: StatusResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsBlacklisted {
                address: String::from(blacklistee_address),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.status);

    // test if blacklisted, blacklistee cannot send coins of denom
    let sudo_msg = SudoMsg::BeforeSend {
        from: blacklistee_address.to_string(),
        to: "anyone".to_string(),
        amount: coins(1000, denom.clone()),
    };
    let res = contract::sudo(deps.as_mut(), mock_env(), sudo_msg);
    let err = res.unwrap_err();
    match err {
        ContractError::Blacklisted {
            address: _blacklistee,
        } => {}
        _ => {
            panic!("should be blacklisted, but is {}", err)
        }
    }

    // test if blacklisted, blacklistee cannot receive coins of denom
    let sudo_msg = SudoMsg::BeforeSend {
        from: "anyone".to_string(),
        to: blacklistee_address.to_string(),
        amount: coins(1000, denom.clone()),
    };
    let res = contract::sudo(deps.as_mut(), mock_env(), sudo_msg);
    let err = res.unwrap_err();
    match err {
        ContractError::Blacklisted {
            address: _blacklistee,
        } => {}
        _ => {
            panic!("should be blacklisted, but is {}", err)
        }
    }

    // test if blacklisted, blacklistee cannot send coins of multiple denom
    contract::sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::BeforeSend {
            from: blacklistee_address.to_string(),
            to: "anyone".to_string(),
            amount: vec![coin(1000, "somethingelse"), coin(1000, denom)],
        },
    )
    .unwrap_err();

    // admin can remove blacklisting capabilitity
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetBlacklister {
            address: String::from(blacklister_address),
            status: false,
        },
    )
    .unwrap();

    // query blacklister status is false
    let res: StatusResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsBlacklister {
                address: String::from(blacklister_address),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(!res.status);

    // make sure freezer can no longer unblacklist
    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(blacklister_address, &[]),
        ExecuteMsg::Blacklist {
            address: String::from(blacklistee_address),
            status: false,
        },
    )
    .unwrap_err();
    match err {
        ContractError::Unauthorized {} => {}
        _ => panic!(
            "False blacklister should generate a unauthorized error, but got {}",
            err
        ),
    }
}

#[test]
fn minting() {
    let mut deps = mock_dependencies();

    let (_, denom) = initialize_contract(deps.as_mut());

    let minter = "minter";

    // tests if the contract throws an error for minting by unauthorized minters
    let unauthorized_info = mock_info("anyone", &[]);
    contract::execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::Mint {
            to_address: String::from(minter),
            amount: Uint128::from(100u64),
        },
    )
    .unwrap_err();

    // tests if the contract throws the right error for add_minters by non-admin
    let unauthorized_info = mock_info("anyone", &[]);
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::SetMinter {
            address: String::from(minter),
            allowance: Uint128::from(1000u64),
        },
    );
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // admin adds minter with allowance of 1000
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetMinter {
            address: String::from(minter),
            allowance: Uint128::from(1000u64),
        },
    )
    .unwrap();

    // query minter allowance
    let res: AllowanceResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::MintAllowance {
                address: String::from(minter),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.allowance == 1000);

    // new minter can mint 500 coins
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(minter, &[]),
        ExecuteMsg::Mint {
            to_address: String::from(minter),
            amount: Uint128::from(500u64),
        },
    )
    .unwrap();
    assert!(res.messages.len() == 2);

    // query that minter allowance should have gone down to 500 remaining
    let res: AllowanceResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::MintAllowance {
                address: String::from(minter),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.allowance == 500);

    // make sure that minter can't mint more than remaining allowance
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(minter, &[]),
        ExecuteMsg::Mint {
            to_address: String::from(minter),
            amount: Uint128::from(600u64),
        },
    )
    .unwrap_err();

    // admin can adjust minting allowance
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetMinter {
            address: String::from(minter),
            allowance: Uint128::from(100000u64),
        },
    )
    .unwrap();

    // query minter allowance
    let res: AllowanceResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::MintAllowance {
                address: String::from(minter),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.allowance == 100000);
}

#[test]
fn burning() {
    let mut deps = mock_dependencies();

    let (_, denom) = initialize_contract(deps.as_mut());

    let burner = "burner";

    // tests if the contract throws an error for burning by unauthorized burners
    let unauthorized_info = mock_info("anyone", &[]);
    contract::execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::Burn {
            amount: Uint128::from(100u64),
        },
    )
    .unwrap_err();

    // tests if the contract throws the right error for add_burners by non-admin
    let unauthorized_info = mock_info("anyone", &[]);
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::SetBurner {
            address: String::from(burner),
            allowance: Uint128::from(1000u64),
        },
    );
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // admin adds burner with allowance of 1000
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetBurner {
            address: String::from(burner),
            allowance: Uint128::from(1000u64),
        },
    )
    .unwrap();

    // query burner allowance
    let res: AllowanceResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BurnAllowance {
                address: String::from(burner),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.allowance == 1000);

    // new burner can burn 500 coins
    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(burner, &[]),
        ExecuteMsg::Burn {
            amount: Uint128::from(500u64),
        },
    )
    .unwrap();
    assert!(res.messages.len() == 1);

    // query that burner allowance should have gone down to 500 remaining
    let res: AllowanceResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BurnAllowance {
                address: String::from(burner),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.allowance == 500);

    // make sure that burner can't burn more than remaining allowance
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(burner, &[]),
        ExecuteMsg::Burn {
            amount: Uint128::from(600u64),
        },
    )
    .unwrap_err();

    // admin can adjust burning allowance
    contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR_ADDRESS, &[]),
        ExecuteMsg::SetBurner {
            address: String::from(burner),
            allowance: Uint128::from(100000u64),
        },
    )
    .unwrap();

    // query burner allowance
    let res: AllowanceResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BurnAllowance {
                address: String::from(burner),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.allowance == 100000);
}
