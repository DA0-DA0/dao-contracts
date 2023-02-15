use std::borrow::BorrowMut;

use crate::msg::{
    ExecuteMsg, InstantiateMsg, ListStakersResponse, QueryMsg, StakedBalanceAtHeightResponse,
    StakedValueResponse, StakerBalanceResponse, TotalStakedAtHeightResponse, TotalValueResponse,
};
use crate::state::Config;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, Addr, Coin, Empty, Uint128};
use cw_controllers::ClaimsResponse;
use cw_multi_test::{
    custom_app, next_block, App, AppResponse, Contract, ContractWrapper, Executor,
};
use cw_utils::Duration;

const DAO_ADDR: &str = "dao";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const DENOM: &str = "ujuno";
const INVALID_DENOM: &str = "uinvalid";

fn query_staked_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = QueryMsg::StakedBalanceAtHeight {
        address: address.into(),
        height: None,
    };
    let result: StakedBalanceAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn query_staked_value<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = QueryMsg::StakedValue {
        address: address.into(),
    };
    let result: StakedValueResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.value
}

fn query_total_value<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
    let msg = QueryMsg::TotalValue {};
    let result: TotalValueResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.total
}

fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
    let msg = QueryMsg::TotalStakedAtHeight { height: None };
    let result: TotalStakedAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.total
}

fn staking_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    custom_app(|r, _a, s| {
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(DAO_ADDR),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADDR1),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADDR2),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                ],
            )
            .unwrap();
    })
}

fn instantiate_staking(app: &mut App, staking_id: u64, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(
        staking_id,
        Addr::unchecked(DAO_ADDR),
        &msg,
        &[],
        "Staking",
        None,
    )
    .unwrap()
}

fn stake_tokens(
    app: &mut App,
    staking_addr: &Addr,
    sender: &str,
    amount: u128,
    denom: &str,
) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        staking_addr.clone(),
        &ExecuteMsg::Stake {},
        &coins(amount, denom),
    )
}

fn unstake_tokens(
    app: &mut App,
    staking_addr: &Addr,
    sender: &str,
    amount: u128,
) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        staking_addr.clone(),
        &ExecuteMsg::Unstake {
            amount: Uint128::new(amount),
        },
        &[],
    )
}

fn claim(app: &mut App, staking_addr: Addr, sender: &str) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        staking_addr,
        &ExecuteMsg::Claim {},
        &[],
    )
}

fn update_config(
    app: &mut App,
    staking_addr: Addr,
    sender: &str,
    owner: Option<String>,
    manager: Option<String>,
    duration: Option<Duration>,
) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        staking_addr,
        &ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        },
        &[],
    )
}

fn get_config(app: &mut App, staking_addr: Addr) -> Config {
    app.wrap()
        .query_wasm_smart(staking_addr, &QueryMsg::GetConfig {})
        .unwrap()
}

fn get_claims(app: &mut App, staking_addr: Addr, address: String) -> ClaimsResponse {
    app.wrap()
        .query_wasm_smart(staking_addr, &QueryMsg::Claims { address })
        .unwrap()
}

fn get_balance(app: &App, address: &str, denom: &str) -> Uint128 {
    app.wrap().query_balance(address, denom).unwrap().amount
}

#[test]
fn test_instantiate() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    // Populated fields
    let _addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Non populated fields
    let _addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: None,
            manager: None,
            denom: DENOM.to_string(),
            unstaking_duration: None,
        },
    );
}

#[test]
fn test_instantiate_dao_owner() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    // Populated fields
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    let config = get_config(&mut app, addr);

    assert_eq!(config.owner, Some(Addr::unchecked(DAO_ADDR)))
}

#[test]
#[should_panic(expected = "Invalid unstaking duration, unstaking duration cannot be 0")]
fn test_instantiate_invalid_unstaking_duration() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    // Populated fields
    let _addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(0)),
        },
    );

    // Non populated fields
    let _addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: None,
            manager: None,
            denom: DENOM.to_string(),
            unstaking_duration: None,
        },
    );
}

#[test]
#[should_panic(expected = "Must send reserve token 'ujuno'")]
fn test_stake_invalid_denom() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Try and stake an invalid denom
    stake_tokens(&mut app, &addr, ADDR1, 100, INVALID_DENOM).unwrap();
}

#[test]
fn test_stake_valid_denom() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Try and stake an valid denom
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);
}

#[test]
#[should_panic(expected = "Can only unstake less than or equal to the amount you have staked")]
fn test_unstake_none_staked() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    unstake_tokens(&mut app, &addr, ADDR1, 100).unwrap();
}

#[test]
#[should_panic(expected = "Can only unstake less than or equal to the amount you have staked")]
fn test_unstake_invalid_balance() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Stake some tokens
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);

    // Try and unstake too many
    unstake_tokens(&mut app, &addr, ADDR1, 200).unwrap();
}

#[test]
fn test_unstake() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Stake some tokens
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);

    // Unstake some
    unstake_tokens(&mut app, &addr, ADDR1, 75).unwrap();

    // Query claims
    let claims = get_claims(&mut app, addr.clone(), ADDR1.to_string());
    assert_eq!(claims.claims.len(), 1);
    app.update_block(next_block);

    // Unstake the rest
    unstake_tokens(&mut app, &addr, ADDR1, 25).unwrap();

    // Query claims
    let claims = get_claims(&mut app, addr, ADDR1.to_string());
    assert_eq!(claims.claims.len(), 2);
}

#[test]
fn test_unstake_no_unstaking_duration() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: None,
        },
    );

    // Stake some tokens
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);

    // Unstake some tokens
    unstake_tokens(&mut app, &addr, ADDR1, 75).unwrap();

    app.update_block(next_block);

    let balance = get_balance(&mut app, ADDR1, DENOM);
    // 10000 (initial bal) - 100 (staked) + 75 (unstaked) = 9975
    assert_eq!(balance, Uint128::new(9975));

    // Unstake the rest
    unstake_tokens(&mut app, &addr, ADDR1, 25).unwrap();

    let balance = get_balance(&mut app, ADDR1, DENOM);
    // 10000 (initial bal) - 100 (staked) + 75 (unstaked 1) + 25 (unstaked 2) = 10000
    assert_eq!(balance, Uint128::new(10000))
}

#[test]
#[should_panic(expected = "Nothing to claim")]
fn test_claim_no_claims() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    claim(&mut app, addr, ADDR1).unwrap();
}

#[test]
#[should_panic(expected = "Nothing to claim")]
fn test_claim_claim_not_reached() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Stake some tokens
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);

    // Unstake them to create the claims
    unstake_tokens(&mut app, &addr, ADDR1, 100).unwrap();
    app.update_block(next_block);

    // We have a claim but it isnt reached yet so this will still fail
    claim(&mut app, addr, ADDR1).unwrap();
}

#[test]
fn test_claim() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Stake some tokens
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);

    // Unstake some to create the claims
    unstake_tokens(&mut app, &addr, ADDR1, 75).unwrap();
    app.update_block(|b| {
        b.height += 5;
        b.time = b.time.plus_seconds(25);
    });

    // Claim
    claim(&mut app, addr.clone(), ADDR1).unwrap();

    // Query balance
    let balance = get_balance(&mut app, ADDR1, DENOM);
    // 10000 (initial bal) - 100 (staked) + 75 (unstaked) = 9975
    assert_eq!(balance, Uint128::new(9975));

    // Unstake the rest
    unstake_tokens(&mut app, &addr, ADDR1, 25).unwrap();
    app.update_block(|b| {
        b.height += 10;
        b.time = b.time.plus_seconds(50);
    });

    // Claim
    claim(&mut app, addr, ADDR1).unwrap();

    // Query balance
    let balance = get_balance(&mut app, ADDR1, DENOM);
    // 10000 (initial bal) - 100 (staked) + 75 (unstaked 1) + 25 (unstaked 2) = 10000
    assert_eq!(balance, Uint128::new(10000));
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_update_config_invalid_sender() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // From ADDR2, so not owner or manager
    update_config(
        &mut app,
        addr,
        ADDR2,
        Some(ADDR1.to_string()),
        Some(DAO_ADDR.to_string()),
        Some(Duration::Height(10)),
    )
    .unwrap();
}

#[test]
#[should_panic(expected = "Only owner can change owner")]
fn test_update_config_non_owner_changes_owner() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // ADDR1 is the manager so cannot change the owner
    update_config(&mut app, addr, ADDR1, Some(ADDR2.to_string()), None, None).unwrap();
}

#[test]
fn test_update_config_as_owner() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Swap owner and manager, change duration
    update_config(
        &mut app,
        addr.clone(),
        DAO_ADDR,
        Some(ADDR1.to_string()),
        Some(DAO_ADDR.to_string()),
        Some(Duration::Height(10)),
    )
    .unwrap();

    let config = get_config(&mut app, addr);
    assert_eq!(
        Config {
            owner: Some(Addr::unchecked(ADDR1)),
            manager: Some(Addr::unchecked(DAO_ADDR)),
            unstaking_duration: Some(Duration::Height(10)),
            denom: DENOM.to_string(),
        },
        config
    );
}

#[test]
fn test_update_config_as_manager() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Change duration and manager as manager cannot change owner
    update_config(
        &mut app,
        addr.clone(),
        ADDR1,
        Some(DAO_ADDR.to_string()),
        Some(ADDR2.to_string()),
        Some(Duration::Height(10)),
    )
    .unwrap();

    let config = get_config(&mut app, addr);
    assert_eq!(
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR2)),
            unstaking_duration: Some(Duration::Height(10)),
            denom: DENOM.to_string(),
        },
        config
    );
}

#[test]
#[should_panic(expected = "Invalid unstaking duration, unstaking duration cannot be 0")]
fn test_update_config_invalid_duration() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // Change duration and manager as manager cannot change owner
    update_config(
        &mut app,
        addr,
        ADDR1,
        Some(DAO_ADDR.to_string()),
        Some(ADDR2.to_string()),
        Some(Duration::Height(0)),
    )
    .unwrap();
}

#[test]
fn test_query_claims() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    let claims = get_claims(&mut app, addr.clone(), ADDR1.to_string());
    assert_eq!(claims.claims.len(), 0);

    // Stake some tokens
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();
    app.update_block(next_block);

    // Unstake some tokens
    unstake_tokens(&mut app, &addr, ADDR1, 25).unwrap();
    app.update_block(next_block);

    let claims = get_claims(&mut app, addr.clone(), ADDR1.to_string());
    assert_eq!(claims.claims.len(), 1);

    unstake_tokens(&mut app, &addr, ADDR1, 25).unwrap();
    app.update_block(next_block);

    let claims = get_claims(&mut app, addr, ADDR1.to_string());
    assert_eq!(claims.claims.len(), 2);
}

#[test]
fn test_query_get_config() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    let config = get_config(&mut app, addr);
    assert_eq!(
        config,
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR1)),
            unstaking_duration: Some(Duration::Height(5)),
            denom: DENOM.to_string(),
        }
    )
}

#[test]
fn test_query_list_stakers() {
    let mut app = mock_app();
    let staking_id = app.store_code(staking_contract());
    let addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: Some(Duration::Height(5)),
        },
    );

    // ADDR1 stakes
    stake_tokens(&mut app, &addr, ADDR1, 100, DENOM).unwrap();

    // ADDR2 stakes
    stake_tokens(&mut app, &addr, ADDR2, 50, DENOM).unwrap();

    // check entire result set
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            addr.clone(),
            &QueryMsg::ListStakers {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let test_res = ListStakersResponse {
        stakers: vec![
            StakerBalanceResponse {
                address: ADDR1.to_string(),
                balance: Uint128::new(100),
            },
            StakerBalanceResponse {
                address: ADDR2.to_string(),
                balance: Uint128::new(50),
            },
        ],
    };

    assert_eq!(stakers, test_res);

    // skipped 1, check result
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            addr.clone(),
            &QueryMsg::ListStakers {
                start_after: Some(ADDR1.to_string()),
                limit: None,
            },
        )
        .unwrap();

    let test_res = ListStakersResponse {
        stakers: vec![StakerBalanceResponse {
            address: ADDR2.to_string(),
            balance: Uint128::new(50),
        }],
    };

    assert_eq!(stakers, test_res);

    // skipped 2, check result. should be nothing
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            addr,
            &QueryMsg::ListStakers {
                start_after: Some(ADDR2.to_string()),
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(stakers, ListStakersResponse { stakers: vec![] });
}

fn mock_compounding_app() -> App {
    custom_app(|r, _a, s| {
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(DAO_ADDR),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADDR1),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(1000),
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADDR2),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(0),
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: Uint128::new(10000),
                    },
                ],
            )
            .unwrap();
    })
}

#[test]
fn test_auto_compounding_staking() {
    let _deps = mock_dependencies();
    let mut app = mock_compounding_app();

    let _env = mock_env();
    let staking_id = app.store_code(staking_contract());
    app.update_block(next_block);
    let staking_addr = instantiate_staking(
        &mut app,
        staking_id,
        InstantiateMsg {
            owner: Some(DAO_ADDR.to_string()),
            manager: Some(ADDR1.to_string()),
            denom: DENOM.to_string(),
            unstaking_duration: None,
        },
    );
    app.update_block(next_block);
    // Successful bond
    stake_tokens(&mut app, &staking_addr, &ADDR1, 100_u128, DENOM).unwrap();
    app.update_block(next_block);
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(100u128),
        "Staked balance should be 100"
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(100u128),
        "Total staked balance should be 100"
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(100u128),
        "Staked value should be 100"
    );
    assert_eq!(
        query_total_value(&app, &staking_addr),
        Uint128::from(100u128),
        "Total value should be 100"
    );
    assert_eq!(get_balance(&mut app, &ADDR1, DENOM), Uint128::from(900u128));

    // Add compounding rewards
    let _res = app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(ADDR1),
            staking_addr.clone(),
            &ExecuteMsg::Fund {},
            &coins(100_u128, DENOM),
        )
        .unwrap();
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(100u128),
        "Staked balance should be 100 after compounding"
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(100u128),
        "Total staked balance should be 100 after compounding"
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(200u128),
        "Staked value should be 200 after compounding"
    );
    assert_eq!(
        query_total_value(&app, &staking_addr),
        Uint128::from(200u128),
        "Total value should be 200 after compounding"
    );
    assert_eq!(
        get_balance(&mut app, &ADDR1, DENOM),
        Uint128::from(800u128),
        "Balance should be 800 after compounding"
    );

    // Sucessful transfer of unbonded amount
    let _res = app
        .borrow_mut()
        .execute(
            Addr::unchecked(ADDR1),
            cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                amount: coins(100_u128, DENOM),
                to_address: ADDR2.to_string(),
            }),
        )
        .unwrap();

    assert_eq!(
        get_balance(&mut app, ADDR1, DENOM),
        Uint128::from(700u128),
        "Balance should be 700 after transfer"
    );
    assert_eq!(
        get_balance(&mut app, ADDR2, DENOM),
        Uint128::from(100u128),
        "Balance should be 100 after transfer"
    );

    // Addr 2 successful bond
    stake_tokens(
        &mut app,
        &staking_addr,
        &ADDR2,
        Uint128::new(100).u128(),
        DENOM,
    )
    .unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2),
        Uint128::from(50u128),
        "Staked balance should be 50"
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(150u128),
        "Total staked balance should be 150"
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, ADDR2.to_string()),
        Uint128::from(100u128),
        "Staked value should be 100"
    );
    assert_eq!(
        query_total_value(&app, &staking_addr),
        Uint128::from(300u128),
        "Total value should be 300"
    );
    assert_eq!(
        get_balance(&mut app, ADDR2, DENOM),
        Uint128::zero(),
        "Balance should be 0 after staking"
    );

    // Can't unstake more than you have staked
    let _info = mock_info(ADDR2, &[]);
    let _err = unstake_tokens(&mut app, &staking_addr, ADDR2, Uint128::new(51).u128()).unwrap_err();

    // Add compounding rewards
    let _res = app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(ADDR1),
            staking_addr.clone(),
            &ExecuteMsg::Fund {},
            &coins(90_u128, DENOM),
        )
        .unwrap();

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(100u128),
        "Staked balance should be 100 after compounding the second time"
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2),
        Uint128::from(50u128),
        "Staked balance should be 50 after compounding the second time"
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(150u128),
        "Total staked balance should be 150 after compounding the second time"
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(260u128),
        "Staked value should be 260 after compounding the second time"
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, ADDR2.to_string()),
        Uint128::from(130u128),
        "Staked value should be 130 after compounding the second time"
    );
    assert_eq!(
        query_total_value(&app, &staking_addr),
        Uint128::from(390u128),
        "Total value should be 390 after compounding the second time"
    );
    assert_eq!(
        get_balance(&app, ADDR1, DENOM),
        Uint128::from(610u128),
        "Balance should be 610 after compounding the second time"
    );

    // Successful unstake
    let _res = unstake_tokens(&mut app, &staking_addr, ADDR2, Uint128::new(25).u128()).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2),
        Uint128::from(25u128),
        "Staked balance should be 25 after unstaking"
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(125u128),
        "Total staked balance should be 125 after unstaking"
    );
    assert_eq!(
        get_balance(&app, ADDR2, DENOM),
        Uint128::from(65u128),
        "Balance should be 65 after unstaking"
    );
}
