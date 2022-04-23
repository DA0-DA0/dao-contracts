use cosmwasm_std::{Addr, Empty, Uint128};
use cw721::ContractInfoResponse;
use cw_core_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{InstantiateMsg, QueryMsg};

const DAO_ADDR: &str = "dao";
const CREATOR_ADDR: &str = "creator";

fn cw721_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

fn balance_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn instantiate_voting(app: &mut App, voting_id: u64, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(
        voting_id,
        Addr::unchecked(DAO_ADDR),
        &msg,
        &[],
        "voting module",
        None,
    )
    .unwrap()
}

fn proper_instantiate(app: &mut App) -> (Addr, Addr) {
    let cw721_id = app.store_code(cw721_contract());
    let voting_id = app.store_code(balance_voting_contract());

    let token_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(CREATOR_ADDR),
            &cw721_base::msg::InstantiateMsg {
                name: "DAO".to_string(),
                symbol: "DAO".to_string(),
                minter: CREATOR_ADDR.to_string(),
            },
            &[],
            "NFT Voting",
            None,
        )
        .unwrap();

    let voting_addr = instantiate_voting(
        app,
        voting_id,
        InstantiateMsg {
            token_info: crate::msg::TokenInfo::Existing {
                address: token_addr.to_string(),
            },
        },
    );
    (token_addr, voting_addr)
}

#[test]
fn test_existing_nft_info() {
    let mut app = App::default();
    let (initial_token_addr, voting_addr) = proper_instantiate(&mut app);

    let token_address: Addr = app
        .wrap()
        .query_wasm_smart(voting_addr, &QueryMsg::TokenContract {})
        .unwrap();

    assert_eq!(token_address, initial_token_addr);

    let token_info: ContractInfoResponse = app
        .wrap()
        .query_wasm_smart(initial_token_addr, &cw721::Cw721QueryMsg::ContractInfo {})
        .unwrap();

    assert_eq!(
        token_info,
        ContractInfoResponse {
            name: "DAO".to_string(),
            symbol: "DAO".to_string()
        }
    );
}

#[test]
fn test_existing_nft_voting_power_at_height() {
    let mut app = App::default();
    let (token_addr, voting_addr) = proper_instantiate(&mut app);

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: CREATOR_ADDR.to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(0u64),
            height: app.block_info().height,
        }
    );

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_addr.clone(),
        &cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id: "DAO1".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_addr.clone(),
        &cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id: "DAO2".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: CREATOR_ADDR.to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(2u64),
            height: app.block_info().height,
        }
    );

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_addr,
        &cw721::Cw721ExecuteMsg::TransferNft {
            recipient: DAO_ADDR.to_string(),
            token_id: "DAO1".to_string(),
        },
        &[],
    )
    .unwrap();

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: CREATOR_ADDR.to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(1u64),
            height: app.block_info().height,
        }
    );

    let dao_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr,
            &QueryMsg::VotingPowerAtHeight {
                address: DAO_ADDR.to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        dao_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(1u64),
            height: app.block_info().height,
        }
    );
}

#[test]
fn test_existing_nft_total_voting_power_at_height() {
    let mut app = App::default();
    let (token_addr, voting_addr) = proper_instantiate(&mut app);

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_addr.clone(),
        &cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id: "DAO1".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_addr.clone(),
        &cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id: "DAO2".to_string(),
            owner: CREATOR_ADDR.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_addr,
        &cw721::Cw721ExecuteMsg::TransferNft {
            recipient: DAO_ADDR.to_string(),
            token_id: "DAO1".to_string(),
        },
        &[],
    )
    .unwrap();

    let total_voting_power: TotalPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(voting_addr, &QueryMsg::TotalPowerAtHeight { height: None })
        .unwrap();

    assert_eq!(
        total_voting_power,
        TotalPowerAtHeightResponse {
            power: Uint128::from(2u64),
            height: app.block_info().height,
        }
    );
}
