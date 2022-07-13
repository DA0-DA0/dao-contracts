use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::msg::{
    ExecuteMsg, Owner, QueryMsg, StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
};
use crate::state::{Config, MAX_CLAIMS};
use crate::ContractError;
use anyhow::Result as AnyResult;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{to_binary, Addr, Empty, MessageInfo, Uint128};
use cw721_controllers::NftClaim;
use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
use cw_utils::Duration;
use cw_utils::Expiration::AtHeight;
use std::borrow::BorrowMut;
use std::convert::TryFrom;

const ADDR1: &str = "addr0001";
const ADDR2: &str = "addr0002";
const ADDR3: &str = "addr0003";
const ADDR4: &str = "addr0004";
const NFT_ID1: &str = "fake_nft1";
const NFT_ID2: &str = "fake_nft2";
const NFT_ID3: &str = "fake_nft3";
const NFT_ID4: &str = "fake_nft4";

fn contract_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    App::default()
}

fn get_nft_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = cw721::Cw721QueryMsg::Tokens {
        owner: address.into(),
        start_after: None,
        limit: None,
    };
    let result: cw721::TokensResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    Uint128::from(u128::try_from(result.tokens.len()).unwrap())
}

fn instantiate_cw721(app: &mut App) -> Addr {
    let cw721_id = app.store_code(contract_cw721());
    let msg = cw721_base::msg::InstantiateMsg {
        name: "Test".to_string(),
        symbol: "Test".to_string(),
        minter: ADDR1.to_string(),
    };

    app.instantiate_contract(cw721_id, Addr::unchecked(ADDR1), &msg, &[], "cw721", None)
        .unwrap()
}

fn instantiate_staking(app: &mut App, cw721: Addr, unstaking_duration: Option<Duration>) -> Addr {
    let staking_code_id = app.store_code(contract_staking());
    let msg = crate::msg::InstantiateMsg {
        owner: Some(Owner::Addr("owner".to_string())),
        manager: Some("manager".to_string()),
        nft_address: cw721.to_string(),
        unstaking_duration,
    };
    app.instantiate_contract(
        staking_code_id,
        Addr::unchecked(ADDR1),
        &msg,
        &[],
        "staking",
        None,
    )
    .unwrap()
}

fn setup_test_case(app: &mut App, unstaking_duration: Option<Duration>) -> (Addr, Addr) {
    // Instantiate cw721 contract
    let cw721_addr = instantiate_cw721(app);
    app.update_block(next_block);
    // Instantiate staking contract
    let staking_addr = instantiate_staking(app, cw721_addr.clone(), unstaking_duration);
    app.update_block(next_block);
    (staking_addr, cw721_addr)
}

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

fn query_voting_power<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
    height: Option<u64>,
) -> Uint128 {
    let msg = QueryMsg::VotingPowerAtHeight {
        height,
        address: address.into(),
    };
    let result: cw_core_interface::voting::VotingPowerAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.power
}

fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> Config {
    let msg = QueryMsg::GetConfig {};
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
    let msg = QueryMsg::TotalStakedAtHeight { height: None };
    let result: TotalStakedAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.total
}

fn query_total_power_at_height<T: Into<String>>(
    app: &App,
    contract_addr: T,
    height: Option<u64>,
) -> Uint128 {
    let msg = QueryMsg::TotalPowerAtHeight { height };
    let result: cw_core_interface::voting::TotalPowerAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.power
}

fn query_nft_claims<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Vec<NftClaim> {
    let msg = QueryMsg::NftClaims {
        address: address.into(),
    };
    let result: cw721_controllers::NftClaimsResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.nft_claims
}

fn mint_nft(
    app: &mut App,
    cw721_addr: &Addr,
    token_id: String,
    recipient: String,
    info: MessageInfo,
) -> AnyResult<AppResponse> {
    let msg = cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
        token_id,
        owner: recipient,
        token_uri: None,
        extension: None,
    });
    app.execute_contract(info.sender, cw721_addr.clone(), &msg, &[])
}

fn stake_nft(
    app: &mut App,
    staking_addr: &Addr,
    cw721_addr: &Addr,
    token_id: String,
    info: MessageInfo,
) -> AnyResult<AppResponse> {
    let msg = cw721::Cw721ExecuteMsg::SendNft {
        contract: staking_addr.to_string(),
        token_id,
        msg: to_binary("Test").unwrap(),
    };
    app.execute_contract(info.sender, cw721_addr.clone(), &msg, &[])
}

fn update_config(
    app: &mut App,
    staking_addr: &Addr,
    info: MessageInfo,
    owner: Option<Addr>,
    manager: Option<Addr>,
    duration: Option<Duration>,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig {
        owner: owner.map(|a| a.to_string()),
        manager: manager.map(|a| a.to_string()),
        duration,
    };
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

fn unstake_tokens(
    app: &mut App,
    staking_addr: &Addr,
    info: MessageInfo,
    token_ids: Vec<String>,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Unstake { token_ids };
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

fn claim_nfts(app: &mut App, staking_addr: &Addr, info: MessageInfo) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::ClaimNfts {};
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

#[test]
fn test_update_config() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let (staking_addr, _cw721_addr) = setup_test_case(&mut app, None);

    let info = mock_info("owner", &[]);
    let _env = mock_env();
    // Test update admin
    update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("owner2")),
        None,
        Some(Duration::Height(100)),
    )
    .unwrap();

    let config = query_config(&app, &staking_addr);
    assert_eq!(config.owner, Some(Addr::unchecked("owner2".to_string())));
    assert_eq!(config.unstaking_duration, Some(Duration::Height(100)));

    // Try updating owner with original owner, which is now invalid
    let info = mock_info("owner", &[]);
    let _err = update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("owner3")),
        None,
        Some(Duration::Height(100)),
    )
    .unwrap_err();

    // Add manager
    let info = mock_info("owner2", &[]);
    let _env = mock_env();
    update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("owner2")),
        Some(Addr::unchecked("manager")),
        Some(Duration::Height(100)),
    )
    .unwrap();

    let config = query_config(&app, &staking_addr);

    assert_eq!(config.owner, Some(Addr::unchecked("owner2".to_string())));
    assert_eq!(config.manager, Some(Addr::unchecked("manager".to_string())));

    // Manager can update unstaking duration
    let info = mock_info("manager", &[]);
    let _env = mock_env();
    update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("owner2")),
        Some(Addr::unchecked("manager")),
        Some(Duration::Height(50)),
    )
    .unwrap();
    let config = query_config(&app, &staking_addr);
    assert_eq!(config.owner, Some(Addr::unchecked("owner2".to_string())));
    assert_eq!(config.unstaking_duration, Some(Duration::Height(50)));

    // Manager cannot update owner
    let info = mock_info("manager", &[]);
    let _env = mock_env();
    update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("manager")),
        Some(Addr::unchecked("manager")),
        Some(Duration::Height(50)),
    )
    .unwrap_err();

    // Manager can update manager
    let info = mock_info("owner2", &[]);
    let _env = mock_env();
    update_config(
        &mut app,
        &staking_addr,
        info,
        Some(Addr::unchecked("owner2")),
        None,
        Some(Duration::Height(50)),
    )
    .unwrap();

    let config = query_config(&app, &staking_addr);
    assert_eq!(config.owner, Some(Addr::unchecked("owner2".to_string())));
    assert_eq!(config.manager, None);

    // Remove owner
    let info = mock_info("owner2", &[]);
    let _env = mock_env();
    update_config(
        &mut app,
        &staking_addr,
        info,
        None,
        None,
        Some(Duration::Height(100)),
    )
    .unwrap();

    // Assert no further updates can be made
    let info = mock_info("owner2", &[]);
    let _env = mock_env();
    let err: ContractError = update_config(
        &mut app,
        &staking_addr,
        info,
        None,
        None,
        Some(Duration::Height(100)),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    let info = mock_info("manager", &[]);
    let _env = mock_env();
    let err: ContractError = update_config(
        &mut app,
        &staking_addr,
        info,
        None,
        None,
        Some(Duration::Height(100)),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
fn test_instantiate_with_instantiator_owner() {
    let mut app = App::default();
    // Instantiate cw721 contract
    let cw721_addr = instantiate_cw721(&mut app);
    // Instantiate staking contract
    let staking_addr = {
        let staking_code_id = app.store_code(contract_staking());
        let msg = crate::msg::InstantiateMsg {
            owner: Some(Owner::Instantiator {}),
            manager: Some("manager".to_string()),
            nft_address: cw721_addr.to_string(),
            unstaking_duration: None,
        };
        app.instantiate_contract(
            staking_code_id,
            Addr::unchecked(ADDR1),
            &msg,
            &[],
            "staking",
            None,
        )
        .unwrap()
    };

    let config = query_config(&app, staking_addr);
    assert_eq!(config.owner, Some(Addr::unchecked(ADDR1.to_string())))
}

#[test]
fn test_staking() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, cw721_addr) = setup_test_case(&mut app, None);

    // Ensure this is propoerly initialized to zero.
    assert_eq!(
        query_total_power_at_height(&app, &staking_addr, None),
        Uint128::zero()
    );

    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();

    // Successful bond
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID2.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        info.clone(),
    )
    .unwrap();

    let start_block = app.block_info().height;

    // Very important that this balances is not reflected until
    // the next block. This protects us from flash loan hostile
    // takeovers.
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::zero()
    );
    assert_eq!(
        query_voting_power(&app, &staking_addr, ADDR1.to_string(), None),
        Uint128::zero()
    );

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(1u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(1u128)
    );
    assert_eq!(
        query_total_power_at_height(&app, &staking_addr, None),
        Uint128::new(1)
    );

    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1.to_string()),
        Uint128::from(1u128)
    );

    assert_eq!(
        query_voting_power(&app, &staking_addr, ADDR1.to_string(), None),
        Uint128::from(1u128)
    );
    // Back in time query.
    assert_eq!(
        query_voting_power(&app, &staking_addr, ADDR1.to_string(), Some(start_block)),
        Uint128::from(0u128)
    );

    // Can't transfer bonded amount
    let msg = cw721::Cw721ExecuteMsg::TransferNft {
        recipient: ADDR2.to_string(),
        token_id: NFT_ID1.to_string(),
    };

    let _err = app
        .borrow_mut()
        .execute_contract(info.sender.clone(), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // Sucessful transfer of unbonded amount
    let msg = cw721::Cw721ExecuteMsg::TransferNft {
        recipient: ADDR2.to_string(),
        token_id: NFT_ID2.to_string(),
    };
    let _res = app
        .borrow_mut()
        .execute_contract(info.sender, cw721_addr.clone(), &msg, &[])
        .unwrap();

    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(0u128)
    );
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR2),
        Uint128::from(1u128)
    );

    // Addr 2 successful bond
    let info = mock_info(ADDR2, &[]);
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID2.to_string(),
        info,
    )
    .unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2),
        Uint128::from(1u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(2u128)
    );

    // Can't unstake other's staked
    let info = mock_info(ADDR2, &[]);
    let _err =
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap_err();

    // Successful unstake
    let info = mock_info(ADDR2, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2),
        Uint128::from(0u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(1u128)
    );

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1),
        Uint128::from(1u128)
    );
}

#[test]
fn test_info_query() {
    let mut app = mock_app();
    let unstaking_blocks = 1u64;
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, _) = setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));
    let info: cw_core_interface::voting::InfoResponse = app
        .wrap()
        .query_wasm_smart(staking_addr, &QueryMsg::Info {})
        .unwrap();

    assert_eq!(
        info,
        cw_core_interface::voting::InfoResponse {
            info: cw2::ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            }
        }
    )
}

#[test]
fn test_max_claims() {
    let mut app = mock_app();
    let unstaking_blocks = 1u64;
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, cw721_addr) =
        setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));

    let info = mock_info(ADDR1, &[]);

    // Create the max number of claims
    for claim in 0..MAX_CLAIMS {
        mint_nft(
            &mut app,
            &cw721_addr,
            claim.to_string(),
            ADDR1.to_string(),
            info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            claim.to_string(),
            info.clone(),
        )
        .unwrap();
    }
    // Unstake all together.
    unstake_tokens(
        &mut app,
        &staking_addr,
        info.clone(),
        (0..MAX_CLAIMS).map(|i| i.to_string()).collect(),
    )
    .unwrap();

    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        info.clone(),
    )
    .unwrap();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID2.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID2.to_string(),
        info.clone(),
    )
    .unwrap();

    // Additional unstaking attempts ought to fail.
    unstake_tokens(
        &mut app,
        &staking_addr,
        info.clone(),
        vec![NFT_ID1.to_string()],
    )
    .unwrap_err();

    // Clear out the claims list.
    app.update_block(next_block);
    claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();

    // Unstaking now allowed again.
    unstake_tokens(
        &mut app,
        &staking_addr,
        info.clone(),
        vec![NFT_ID1.to_string()],
    )
    .unwrap();
    app.update_block(next_block);
    unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();

    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(10u128)
    );
}

#[test]
fn test_unstaking_with_claims() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let unstaking_blocks = 10u64;
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, cw721_addr) =
        setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));

    let info = mock_info(ADDR1, &[]);

    // Successful bond
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();
    let _res = stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        info,
    )
    .unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1),
        Uint128::from(1u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(1u128)
    );
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(0u128)
    );

    // Unstake
    let info = mock_info(ADDR1, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1),
        Uint128::from(0u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(0u128)
    );
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(0u128)
    );

    // Cannot claim when nothing is available
    let info = mock_info(ADDR1, &[]);
    let _err: ContractError = claim_nfts(&mut app, &staking_addr, info)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(_err, ContractError::NothingToClaim {});

    // Successful claim
    app.update_block(|b| b.height += unstaking_blocks);
    let info = mock_info(ADDR1, &[]);
    claim_nfts(&mut app, &staking_addr, info).unwrap();

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1),
        Uint128::from(0u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(0u128)
    );
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(1u128)
    );
}

#[test]
fn multiple_address_staking() {
    let mut app = mock_app();
    let amount1 = Uint128::from(1u128);
    let unstaking_blocks = 10u64;
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, cw721_addr) =
        setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));

    let minter_info = mock_info(ADDR1, &[]);
    // Successful bond
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        minter_info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        minter_info.clone(),
    )
    .unwrap();
    app.update_block(next_block);

    let info = mock_info(ADDR2, &[]);
    // Successful bond
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID2.to_string(),
        ADDR2.to_string(),
        minter_info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID2.to_string(),
        info,
    )
    .unwrap();
    app.update_block(next_block);

    let info = mock_info(ADDR3, &[]);
    // Successful bond
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID3.to_string(),
        ADDR3.to_string(),
        minter_info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID3.to_string(),
        info,
    )
    .unwrap();
    app.update_block(next_block);

    let info = mock_info(ADDR4, &[]);
    // Successful bond
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID4.to_string(),
        ADDR4.to_string(),
        minter_info,
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID4.to_string(),
        info,
    )
    .unwrap();
    app.update_block(next_block);

    assert_eq!(query_staked_balance(&app, &staking_addr, ADDR1), amount1);
    assert_eq!(query_staked_balance(&app, &staking_addr, ADDR2), amount1);
    assert_eq!(query_staked_balance(&app, &staking_addr, ADDR3), amount1);
    assert_eq!(query_staked_balance(&app, &staking_addr, ADDR4), amount1);

    assert_eq!(
        query_total_staked(&app, &staking_addr),
        amount1.checked_mul(Uint128::new(4)).unwrap()
    );

    assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR1), Uint128::zero());
    assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR2), Uint128::zero());
    assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR3), Uint128::zero());
    assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR4), Uint128::zero());
}

#[test]
fn test_simple_unstaking_with_duration() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, cw721_addr) = setup_test_case(&mut app, Some(Duration::Height(1)));

    // Bond Address 1
    let minter_info = mock_info(ADDR1, &[]);
    let _env = mock_env();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        minter_info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        minter_info.clone(),
    )
    .unwrap();

    // Bond Address 2
    let info = mock_info(ADDR2, &[]);
    let _env = mock_env();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID2.to_string(),
        ADDR2.to_string(),
        minter_info,
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID2.to_string(),
        info,
    )
    .unwrap();
    app.update_block(next_block);
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(1u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(1u128)
    );

    // Unstake Addr1
    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();
    unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap();

    // Unstake Addr2
    let info = mock_info(ADDR2, &[]);
    let _env = mock_env();
    unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(0u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2.to_string()),
        Uint128::from(0u128)
    );

    // Claim
    assert_eq!(
        query_nft_claims(&app, &staking_addr, ADDR1),
        vec![NftClaim {
            token_id: NFT_ID1.to_string(),
            release_at: AtHeight(12349)
        }]
    );
    assert_eq!(
        query_nft_claims(&app, &staking_addr, ADDR2),
        vec![NftClaim {
            token_id: NFT_ID2.to_string(),
            release_at: AtHeight(12349)
        }]
    );

    let info = mock_info(ADDR1, &[]);
    claim_nfts(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(1u128)
    );

    let info = mock_info(ADDR2, &[]);
    claim_nfts(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR2),
        Uint128::from(1u128)
    );
}

#[test]
fn test_simple_unstaking_without_rewards_with_duration() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let _token_address = Addr::unchecked("token_address");
    let (staking_addr, cw721_addr) = setup_test_case(&mut app, Some(Duration::Height(1)));

    // Bond Address 1
    let minter_info = mock_info(ADDR1, &[]);
    let _env = mock_env();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        minter_info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        minter_info.clone(),
    )
    .unwrap();

    // Bond Address 2
    let info = mock_info(ADDR2, &[]);
    let _env = mock_env();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID2.to_string(),
        ADDR2.to_string(),
        minter_info,
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID2.to_string(),
        info,
    )
    .unwrap();
    app.update_block(next_block);
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(1u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(1u128)
    );

    // Unstake Addr1
    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();
    unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap();

    // Unstake Addr2
    let info = mock_info(ADDR2, &[]);
    let _env = mock_env();
    unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
        Uint128::from(0u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, ADDR2.to_string()),
        Uint128::from(0u128)
    );

    // Claim
    assert_eq!(
        query_nft_claims(&app, &staking_addr, ADDR1),
        vec![NftClaim {
            token_id: NFT_ID1.to_string(),
            release_at: AtHeight(12349)
        }]
    );
    assert_eq!(
        query_nft_claims(&app, &staking_addr, ADDR2),
        vec![NftClaim {
            token_id: NFT_ID2.to_string(),
            release_at: AtHeight(12349)
        }]
    );

    let info = mock_info(ADDR1, &[]);
    claim_nfts(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR1),
        Uint128::from(1u128)
    );

    let info = mock_info(ADDR2, &[]);
    claim_nfts(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        get_nft_balance(&app, &cw721_addr, ADDR2),
        Uint128::from(1u128)
    );
}

#[test]
fn test_unstake_that_which_you_do_not_own() {
    let mut app = mock_app();
    let (staking_addr, cw721_addr) = setup_test_case(&mut app, None);

    let info = mock_info(ADDR1, &[]);

    // Mint and stake an NFT for addr1.
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID1.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();
    mint_nft(
        &mut app,
        &cw721_addr,
        NFT_ID2.to_string(),
        ADDR1.to_string(),
        info.clone(),
    )
    .unwrap();

    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID1.to_string(),
        info.clone(),
    )
    .unwrap();
    stake_nft(
        &mut app,
        &staking_addr,
        &cw721_addr,
        NFT_ID2.to_string(),
        info,
    )
    .unwrap();

    app.update_block(next_block);

    let info = mock_info(ADDR2, &[]);
    let err: ContractError =
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()])
            .unwrap_err()
            .downcast()
            .unwrap();

    assert_eq!(err, ContractError::NotStaked {});

    // Try to unstaking the same token more than once as the owner of
    // the token.
    let info = mock_info(ADDR1, &[]);
    let res: ContractError = unstake_tokens(
        &mut app,
        &staking_addr,
        info,
        vec![NFT_ID1.to_string(), NFT_ID1.to_string()],
    )
    .unwrap_err()
    .downcast()
    .unwrap();

    assert_eq!(res, ContractError::NotStaked {});

    let total_staked = query_total_staked(&app, &staking_addr);
    assert_eq!(total_staked, Uint128::new(2));

    // Legally unstake more than one token at once and make sure the
    // count decreases as expected.
    let info = mock_info(ADDR1, &[]);
    unstake_tokens(
        &mut app,
        &staking_addr,
        info,
        vec![NFT_ID1.to_string(), NFT_ID2.to_string()],
    )
    .unwrap();

    app.update_block(next_block);

    let voting_power = query_voting_power(&app, &staking_addr, ADDR1, None);
    assert_eq!(voting_power, Uint128::zero());

    let total_staked = query_total_staked(&app, &staking_addr);
    assert_eq!(total_staked, Uint128::zero());
}
