use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
};
use cosmwasm_std::{
    coins, from_json, Addr, Binary, CosmosMsg, Deps, DepsMut, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::*;
use cw20_base::{
    contract::{query_balance, query_minter, query_token_info},
    msg::InstantiateMsg as Cw20InstantiateMsg,
    ContractError as Cw20ContractError,
};

use crate::{contract::*, msg::*, ContractError};

// TESTS COPIED FROM CW20-BASE

fn get_balance<T: Into<String>>(deps: Deps, address: T) -> Uint128 {
    query_balance(deps, address.into()).unwrap().balance
}

// this will set up the instantiation for other tests
fn do_instantiate_with_minter(
    deps: DepsMut,
    addr: &str,
    amount: Uint128,
    minter: &str,
    cap: Option<Uint128>,
) -> TokenInfoResponse {
    _do_instantiate(
        deps,
        addr,
        amount,
        Some(MinterResponse {
            minter: minter.to_string(),
            cap,
        }),
    )
}

// this will set up the instantiation for other tests
fn do_instantiate(deps: DepsMut, addr: &str, amount: Uint128) -> TokenInfoResponse {
    _do_instantiate(deps, addr, amount, None)
}

// this will set up the instantiation for other tests
fn _do_instantiate(
    mut deps: DepsMut,
    addr: &str,
    amount: Uint128,
    mint: Option<MinterResponse>,
) -> TokenInfoResponse {
    let instantiate_msg = InstantiateMsg {
        owner: Some("owner".to_string()),
        name: "Auto Gen".to_string(),
        symbol: "AUTO".to_string(),
        decimals: 3,
        initial_balances: vec![Cw20Coin {
            address: addr.to_string(),
            amount,
        }],
        mint: mint.clone(),
        marketing: None,
    };
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let res = instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let meta = query_token_info(deps.as_ref()).unwrap();
    assert_eq!(
        meta,
        TokenInfoResponse {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            total_supply: amount,
        }
    );
    assert_eq!(get_balance(deps.as_ref(), addr), amount);
    assert_eq!(query_minter(deps.as_ref()).unwrap(), mint,);
    meta
}

const PNG_HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

mod instantiate {
    use super::*;

    #[test]
    fn basic() {
        let mut deps = mock_dependencies();
        let addr = deps.api.addr_make("addr0000");
        let amount = Uint128::from(11223344u128);
        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20Coin {
                address: addr.to_string(),
                amount,
            }],
            mint: None,
            marketing: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr), Uint128::new(11223344));
    }

    #[test]
    fn mintable() {
        let mut deps = mock_dependencies();
        let addr = deps.api.addr_make("addr0000");
        let amount = Uint128::new(11223344);
        let minter = deps.api.addr_make("asmodat").to_string();
        let limit = Uint128::new(511223344);
        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20Coin {
                address: addr.to_string(),
                amount,
            }],
            mint: Some(MinterResponse {
                minter: minter.clone(),
                cap: Some(limit),
            }),
            marketing: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr), Uint128::new(11223344));
        assert_eq!(
            query_minter(deps.as_ref()).unwrap(),
            Some(MinterResponse {
                minter,
                cap: Some(limit),
            }),
        );
    }

    #[test]
    fn mintable_over_cap() {
        let mut deps = mock_dependencies();
        let amount = Uint128::new(11223344);
        let minter = deps.api.addr_make("asmodat");
        let addr = deps.api.addr_make("addr0000");
        let limit = Uint128::new(11223300);
        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20Coin {
                address: addr.to_string(),
                amount,
            }],
            mint: Some(MinterResponse {
                minter: minter.to_string(),
                cap: Some(limit),
            }),
            marketing: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let err = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Cw20(StdError::generic_err("Initial supply greater than cap").into())
        );
    }

    mod marketing {
        use cw20_base::contract::{query_download_logo, query_marketing_info};

        use super::*;

        #[test]
        fn basic() {
            let mut deps = mock_dependencies();

            let marketing = deps.api.addr_make("marketing");

            let instantiate_msg = InstantiateMsg {
                owner: None,
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![],
                mint: None,
                marketing: Some(InstantiateMarketingInfo {
                    project: Some("Project".to_owned()),
                    description: Some("Description".to_owned()),
                    marketing: Some(marketing.to_string()),
                    logo: Some(Logo::Url("url".to_owned())),
                }),
            };

            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            assert_eq!(
                query_marketing_info(deps.as_ref()).unwrap(),
                MarketingInfoResponse {
                    project: Some("Project".to_owned()),
                    description: Some("Description".to_owned()),
                    marketing: Some(marketing),
                    logo: Some(LogoInfo::Url("url".to_owned())),
                }
            );

            let err = query_download_logo(deps.as_ref()).unwrap_err();
            assert!(
                matches!(err, StdError::NotFound { .. }),
                "Expected StdError::NotFound, received {err}",
            );
        }

        #[test]
        fn invalid_marketing() {
            let mut deps = mock_dependencies();
            let instantiate_msg = InstantiateMsg {
                owner: None,
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![],
                mint: None,
                marketing: Some(InstantiateMarketingInfo {
                    project: Some("Project".to_owned()),
                    description: Some("Description".to_owned()),
                    marketing: Some("m".to_owned()),
                    logo: Some(Logo::Url("url".to_owned())),
                }),
            };

            let info = mock_info("creator", &[]);
            let env = mock_env();
            instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap_err();

            let err = query_download_logo(deps.as_ref()).unwrap_err();
            assert!(
                matches!(err, StdError::NotFound { .. }),
                "Expected StdError::NotFound, received {err}",
            );
        }
    }
}

#[test]
fn can_mint_by_minter() {
    let mut deps = mock_dependencies();

    let genesis = deps.api.addr_make("genesis").to_string();
    let amount = Uint128::new(11223344);
    let minter = deps.api.addr_make("asmodat").to_string();
    let limit = Uint128::new(511223344);
    do_instantiate_with_minter(deps.as_mut(), &genesis, amount, &minter, Some(limit));

    // minter can mint coins to some winner
    let winner = deps.api.addr_make("winner").to_string();
    let prize = Uint128::new(222_222_222);
    let msg = ExecuteMsg::Mint {
        recipient: winner.clone(),
        amount: prize,
    };

    let info = mock_info(minter.as_ref(), &[]);
    let env = mock_env();
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    assert_eq!(get_balance(deps.as_ref(), genesis), amount);
    assert_eq!(get_balance(deps.as_ref(), winner.clone()), prize);

    // Allows minting 0
    let msg = ExecuteMsg::Mint {
        recipient: winner.clone(),
        amount: Uint128::zero(),
    };
    let info = mock_info(minter.as_ref(), &[]);
    let env = mock_env();
    execute(deps.as_mut(), env, info, msg).unwrap();

    // but if it exceeds cap (even over multiple rounds), it fails
    // cap is enforced
    let msg = ExecuteMsg::Mint {
        recipient: winner,
        amount: Uint128::new(333_222_222),
    };
    let info = mock_info(minter.as_ref(), &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Cw20(Cw20ContractError::CannotExceedCap {})
    );
}

#[test]
fn others_cannot_mint() {
    let mut deps = mock_dependencies();

    let genesis = deps.api.addr_make("genesis").to_string();
    let minter = deps.api.addr_make("minter").to_string();
    let winner = deps.api.addr_make("winner").to_string();

    do_instantiate_with_minter(deps.as_mut(), &genesis, Uint128::new(1234), &minter, None);

    let msg = ExecuteMsg::Mint {
        recipient: winner,
        amount: Uint128::new(222),
    };
    let info = mock_info("anyone else", &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Cw20(Cw20ContractError::Unauthorized {}));
}

// MODIFIED FROM cw20-base. Only the owner can update minter now.
#[test]
fn minter_cannot_update_minter() {
    let mut deps = mock_dependencies();

    let genesis = deps.api.addr_make("genesis").to_string();
    let minter = deps.api.addr_make("minter").to_string();

    let cap = Some(Uint128::from(3000000u128));
    do_instantiate_with_minter(deps.as_mut(), &genesis, Uint128::new(1234), &minter, cap);

    let new_minter = deps.api.addr_make("new_minter").to_string();
    let msg = ExecuteMsg::UpdateMinter {
        new_minter: Some(new_minter.clone()),
    };

    let info = mock_info(&minter, &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn others_cannot_update_minter() {
    let mut deps = mock_dependencies();

    let genesis = deps.api.addr_make("genesis").to_string();
    let minter = deps.api.addr_make("minter").to_string();
    let new_minter = deps.api.addr_make("new_minter").to_string();

    do_instantiate_with_minter(deps.as_mut(), &genesis, Uint128::new(1234), &minter, None);

    let msg = ExecuteMsg::UpdateMinter {
        new_minter: Some(new_minter),
    };

    let info = mock_info("not the minter", &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn unset_minter() {
    let mut deps = mock_dependencies();

    let genesis = deps.api.addr_make("genesis").to_string();
    let minter = deps.api.addr_make("minter").to_string();
    let winner = deps.api.addr_make("winner").to_string();

    let cap = None;
    do_instantiate_with_minter(deps.as_mut(), &genesis, Uint128::new(1234), &minter, cap);

    let msg = ExecuteMsg::UpdateMinter { new_minter: None };

    let info = mock_info("owner", &[]);
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());
    let query_minter_msg = QueryMsg::Minter {};
    let res = query(deps.as_ref(), env, query_minter_msg);
    let mint: Option<MinterResponse> = from_json(res.unwrap()).unwrap();

    // Check that mint information was removed.
    assert_eq!(mint, None);

    // Check that old minter can no longer mint.
    let msg = ExecuteMsg::Mint {
        recipient: winner,
        amount: Uint128::new(222),
    };
    let info = mock_info(&minter, &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Cw20(Cw20ContractError::Unauthorized {}));
}

#[test]
fn no_one_mints_if_minter_unset() {
    let mut deps = mock_dependencies();

    let genesis = deps.api.addr_make("genesis").to_string();
    let winner = deps.api.addr_make("winner").to_string();

    do_instantiate(deps.as_mut(), &genesis, Uint128::new(1234));

    let msg = ExecuteMsg::Mint {
        recipient: winner,
        amount: Uint128::new(222),
    };
    let info = mock_info(&genesis, &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Cw20(Cw20ContractError::Unauthorized {}));
}

#[test]
fn instantiate_multiple_accounts() {
    let mut deps = mock_dependencies();
    let amount1 = Uint128::from(11223344u128);
    let addr1 = deps.api.addr_make("addr0001").to_string();
    let amount2 = Uint128::from(7890987u128);
    let addr2 = deps.api.addr_make("addr0002").to_string();
    let info = mock_info("creator", &[]);
    let env = mock_env();

    // Fails with duplicate addresses
    let instantiate_msg = InstantiateMsg {
        owner: None,
        name: "Bash Shell".to_string(),
        symbol: "BASH".to_string(),
        decimals: 6,
        initial_balances: vec![
            Cw20Coin {
                address: addr1.clone(),
                amount: amount1,
            },
            Cw20Coin {
                address: addr1.clone(),
                amount: amount2,
            },
        ],
        mint: None,
        marketing: None,
    };
    let err = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Cw20(Cw20ContractError::DuplicateInitialBalanceAddresses {})
    );

    // Works with unique addresses
    let instantiate_msg = InstantiateMsg {
        owner: None,
        name: "Bash Shell".to_string(),
        symbol: "BASH".to_string(),
        decimals: 6,
        initial_balances: vec![
            Cw20Coin {
                address: addr1.clone(),
                amount: amount1,
            },
            Cw20Coin {
                address: addr2.clone(),
                amount: amount2,
            },
        ],
        mint: None,
        marketing: None,
    };
    let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());
    assert_eq!(
        query_token_info(deps.as_ref()).unwrap(),
        TokenInfoResponse {
            name: "Bash Shell".to_string(),
            symbol: "BASH".to_string(),
            decimals: 6,
            total_supply: amount1 + amount2,
        }
    );
    assert_eq!(get_balance(deps.as_ref(), addr1), amount1);
    assert_eq!(get_balance(deps.as_ref(), addr2), amount2);
}

#[test]
fn queries_work() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    let addr1 = deps.api.addr_make("addr0001").to_string();
    let addr2 = deps.api.addr_make("addr0002").to_string();

    let amount1 = Uint128::from(12340000u128);

    let expected = do_instantiate(deps.as_mut(), &addr1, amount1);

    // check meta query
    let loaded = query_token_info(deps.as_ref()).unwrap();
    assert_eq!(expected, loaded);

    let _info = mock_info("test", &[]);
    let env = mock_env();
    // check balance query (full)
    let data = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Balance { address: addr1 },
    )
    .unwrap();
    let loaded: BalanceResponse = from_json(data).unwrap();
    assert_eq!(loaded.balance, amount1);

    // check balance query (empty)
    let data = query(deps.as_ref(), env, QueryMsg::Balance { address: addr2 }).unwrap();
    let loaded: BalanceResponse = from_json(data).unwrap();
    assert_eq!(loaded.balance, Uint128::zero());
}

#[test]
fn transfer() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let addr1 = deps.api.addr_make("addr0001").to_string();
    let addr2 = deps.api.addr_make("addr0002").to_string();
    let amount1 = Uint128::from(12340000u128);
    let transfer = Uint128::from(76543u128);
    let too_much = Uint128::from(12340321u128);

    do_instantiate(deps.as_mut(), &addr1, amount1);

    // Allows transferring 0
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Transfer {
        recipient: addr2.clone(),
        amount: Uint128::zero(),
    };
    execute(deps.as_mut(), env, info, msg).unwrap();

    // cannot send more than we have
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Transfer {
        recipient: addr2.clone(),
        amount: too_much,
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

    // cannot send from empty account
    let info = mock_info(addr2.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Transfer {
        recipient: addr1.clone(),
        amount: transfer,
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

    // valid transfer
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Transfer {
        recipient: addr2.clone(),
        amount: transfer,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let remainder = amount1.checked_sub(transfer).unwrap();
    assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
    assert_eq!(get_balance(deps.as_ref(), addr2), transfer);
    assert_eq!(
        query_token_info(deps.as_ref()).unwrap().total_supply,
        amount1
    );
}

#[test]
fn burn() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let addr1 = deps.api.addr_make("addr0001").to_string();
    let amount1 = Uint128::from(12340000u128);
    let burn = Uint128::from(76543u128);
    let too_much = Uint128::from(12340321u128);

    do_instantiate(deps.as_mut(), &addr1, amount1);

    // Allows burning 0
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Burn {
        amount: Uint128::zero(),
    };
    execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        query_token_info(deps.as_ref()).unwrap().total_supply,
        amount1
    );

    // cannot burn more than we have
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Burn { amount: too_much };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Cw20(Cw20ContractError::Std(StdError::Overflow { .. }))
    ));
    assert_eq!(
        query_token_info(deps.as_ref()).unwrap().total_supply,
        amount1
    );

    // valid burn reduces total supply
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Burn { amount: burn };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let remainder = amount1.checked_sub(burn).unwrap();
    assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
    assert_eq!(
        query_token_info(deps.as_ref()).unwrap().total_supply,
        remainder
    );
}

#[test]
fn send() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let addr1 = deps.api.addr_make("addr0001").to_string();
    let contract = deps.api.addr_make("contract0001").to_string();
    let amount1 = Uint128::from(12340000u128);
    let transfer = Uint128::from(76543u128);
    let too_much = Uint128::from(12340321u128);
    let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

    do_instantiate(deps.as_mut(), &addr1, amount1);

    // Allows sending 0
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Send {
        contract: contract.clone(),
        amount: Uint128::zero(),
        msg: send_msg.clone(),
    };
    execute(deps.as_mut(), env, info, msg).unwrap();

    // cannot send more than we have
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Send {
        contract: contract.clone(),
        amount: too_much,
        msg: send_msg.clone(),
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

    // valid transfer
    let info = mock_info(addr1.as_ref(), &[]);
    let env = mock_env();
    let msg = ExecuteMsg::Send {
        contract: contract.clone(),
        amount: transfer,
        msg: send_msg.clone(),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 1);

    // ensure proper send message sent
    // this is the message we want delivered to the other side
    let binary_msg = Cw20ReceiveMsg {
        sender: addr1.clone(),
        amount: transfer,
        msg: send_msg,
    }
    .into_binary()
    .unwrap();
    // and this is how it must be wrapped for the vm to process it
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract.clone(),
            msg: binary_msg,
            funds: vec![],
        }))
    );

    // ensure balance is properly transferred
    let remainder = amount1.checked_sub(transfer).unwrap();
    assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
    assert_eq!(get_balance(deps.as_ref(), contract), transfer);
    assert_eq!(
        query_token_info(deps.as_ref()).unwrap().total_supply,
        amount1
    );
}

mod migration {
    use super::*;

    use cosmwasm_std::{to_json_binary, Empty};
    use cw20::{AllAllowancesResponse, AllSpenderAllowancesResponse, SpenderAllowanceInfo};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};
    use cw_utils::Expiration;

    fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn cw20_hooks_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_migrate(crate::contract::migrate);
        Box::new(contract)
    }

    #[test]
    fn test_migrate() {
        let mut app = App::default();

        let sender = app.api().addr_make("sender").to_string();
        let spender = app.api().addr_make("spender").to_string();

        let cw20_base_id = app.store_code(cw20_base_contract());
        let cw20_hooks_id = app.store_code(cw20_hooks_contract());
        let cw20_base_addr = app
            .instantiate_contract(
                cw20_base_id,
                Addr::unchecked("sender"),
                &Cw20InstantiateMsg {
                    name: "Token".to_string(),
                    symbol: "TOKEN".to_string(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: sender.clone(),
                        amount: Uint128::new(100),
                    }],
                    mint: None,
                    marketing: None,
                },
                &[],
                "TOKEN",
                Some(sender.clone()),
            )
            .unwrap();

        // no allowance to start
        let allowance: AllAllowancesResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_base_addr.to_string(),
                &QueryMsg::AllAllowances {
                    owner: sender.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(allowance, AllAllowancesResponse::default());

        // Set allowance
        let allow1 = Uint128::new(7777);
        let expires = Expiration::AtHeight(123_456);
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_base_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::IncreaseAllowance {
                spender: spender.clone(),
                amount: allow1,
                expires: Some(expires),
            })
            .unwrap(),
            funds: vec![],
        });
        app.execute(Addr::unchecked(&sender), msg).unwrap();

        // Now migrate
        app.execute(
            Addr::unchecked(&sender),
            CosmosMsg::Wasm(WasmMsg::Migrate {
                contract_addr: cw20_base_addr.to_string(),
                new_code_id: cw20_hooks_id,
                msg: to_json_binary(&MigrateMsg::FromBase {
                    owner: "owner".to_string(),
                })
                .unwrap(),
            }),
        )
        .unwrap();

        // Smoke check that the contract still works.
        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_base_addr.clone(),
                &QueryMsg::Balance {
                    address: sender.clone(),
                },
            )
            .unwrap();

        assert_eq!(balance.balance, Uint128::new(100));

        // Confirm that the allowance per spender is there
        let allowance: AllSpenderAllowancesResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_base_addr,
                &QueryMsg::AllSpenderAllowances {
                    spender,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(
            allowance.allowances,
            &[SpenderAllowanceInfo {
                owner: sender,
                allowance: allow1,
                expires
            }]
        );
    }
}

mod marketing {
    use cw20_base::contract::{query_download_logo, query_marketing_info};

    use super::*;

    #[test]
    fn update_unauthorised() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");
        let marketing = deps.api.addr_make("marketing");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(marketing.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: Some("New project".to_owned()),
                description: Some("Better description".to_owned()),
                marketing: Some(creator.to_string()),
            },
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});

        // Ensure marketing didn't change
        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(marketing),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_project() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: Some("New project".to_owned()),
                description: None,
                marketing: None,
            },
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("New project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn clear_project() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: Some("".to_owned()),
                description: None,
                marketing: None,
            },
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: None,
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_description() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: None,
                description: Some("Better description".to_owned()),
                marketing: None,
            },
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Better description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn clear_description() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: None,
                description: Some("".to_owned()),
                marketing: None,
            },
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: None,
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_marketing() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");
        let marketing = deps.api.addr_make("marketing");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: None,
                description: None,
                marketing: Some(marketing.to_string()),
            },
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(marketing),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_marketing_invalid() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: None,
                description: None,
                marketing: Some("m".to_owned()),
            },
        )
        .unwrap_err();

        assert!(
            matches!(err, ContractError::Std(_)),
            "Expected Std error, received: {err}",
        );

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn clear_marketing() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateMarketing {
                project: None,
                description: None,
                marketing: Some("".to_owned()),
            },
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: None,
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_logo_url() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Url("new_url".to_owned())),
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("new_url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_logo_png() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Embedded(EmbeddedLogo::Png(PNG_HEADER.into()))),
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Embedded),
            }
        );

        assert_eq!(
            query_download_logo(deps.as_ref()).unwrap(),
            DownloadLogoResponse {
                mime_type: "image/png".to_owned(),
                data: PNG_HEADER.into(),
            }
        );
    }

    #[test]
    fn update_logo_svg() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let img = "<?xml version=\"1.0\"?><svg></svg>".as_bytes();
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Embedded(EmbeddedLogo::Svg(img.into()))),
        )
        .unwrap();

        assert_eq!(res.messages, vec![]);

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Embedded),
            }
        );

        assert_eq!(
            query_download_logo(deps.as_ref()).unwrap(),
            DownloadLogoResponse {
                mime_type: "image/svg+xml".to_owned(),
                data: img.into(),
            }
        );
    }

    #[test]
    fn update_logo_png_oversized() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let img = [&PNG_HEADER[..], &[1; 6000][..]].concat();
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Embedded(EmbeddedLogo::Png(img.into()))),
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Cw20(Cw20ContractError::LogoTooBig {}));

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_logo_svg_oversized() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let img = [
            "<?xml version=\"1.0\"?><svg>",
            std::str::from_utf8(&[b'x'; 6000]).unwrap(),
            "</svg>",
        ]
        .concat()
        .into_bytes();

        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Embedded(EmbeddedLogo::Svg(img.into()))),
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Cw20(Cw20ContractError::LogoTooBig {}));

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_logo_png_invalid() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info(creator.as_str(), &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let img = &[1];
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Embedded(EmbeddedLogo::Png(img.into()))),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::Cw20(Cw20ContractError::InvalidPngHeader {})
        );

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }

    #[test]
    fn update_logo_svg_invalid() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");

        let instantiate_msg = InstantiateMsg {
            owner: None,
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![],
            mint: None,
            marketing: Some(InstantiateMarketingInfo {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator.to_string()),
                logo: Some(Logo::Url("url".to_owned())),
            }),
        };

        let info = mock_info("creator", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let img = &[1];

        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UploadLogo(Logo::Embedded(EmbeddedLogo::Svg(img.into()))),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::Cw20(Cw20ContractError::InvalidXmlPreamble {})
        );

        assert_eq!(
            query_marketing_info(deps.as_ref()).unwrap(),
            MarketingInfoResponse {
                project: Some("Project".to_owned()),
                description: Some("Description".to_owned()),
                marketing: Some(creator),
                logo: Some(LogoInfo::Url("url".to_owned())),
            }
        );

        let err = query_download_logo(deps.as_ref()).unwrap_err();
        assert!(
            matches!(err, StdError::NotFound { .. }),
            "Expected StdError::NotFound, received {err}",
        );
    }
}
