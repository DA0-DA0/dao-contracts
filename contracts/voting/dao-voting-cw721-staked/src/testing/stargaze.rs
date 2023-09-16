use cosmwasm_std::{to_binary, Addr, Empty, Uint128};
use cw721::OwnerOfResponse;
use cw_multi_test::{Contract, ContractWrapper, Executor};
use dao_voting::threshold::ActiveThreshold;
use sg721::{CollectionInfo, RoyaltyInfoResponse, UpdateCollectionInfoMsg};
use sg721_base::msg::CollectionInfoResponse;
use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;

use crate::{
    msg::{InstantiateMsg, NftContract, QueryMsg},
    state::Config,
    testing::CREATOR_ADDR,
};

// Setup Stargaze contracts for multi-test
fn sg721_base_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

// Stargze contracts need a custom message wrapper
fn voting_sg721_staked_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply_empty(crate::contract::reply);
    Box::new(contract)
}

// I can create new Stargaze NFT collection when creating a dao-voting-cw721-staked contract
#[test]
fn test_instantiate_with_new_sg721_collection() -> anyhow::Result<()> {
    let mut app = StargazeApp::default();
    let module_id = app.store_code(voting_sg721_staked_contract());
    let sg721_id = app.store_code(sg721_base_contract());

    let module_addr = app
        .instantiate_contract(
            module_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                nft_contract: NftContract::New {
                    code_id: sg721_id,
                    label: "Test NFT".to_string(),
                    msg: to_binary(&sg721::InstantiateMsg {
                        name: "Test NFT".to_string(),
                        symbol: "TEST".to_string(),
                        minter: CREATOR_ADDR.to_string(),
                        collection_info: CollectionInfo {
                            creator: CREATOR_ADDR.to_string(),
                            description: "Test NFT".to_string(),
                            image: "https://example.com/image.jpg".to_string(),
                            external_link: None,
                            explicit_content: None,
                            start_trading_time: None,
                            royalty_info: None,
                        },
                    })?,
                    initial_nfts: vec![to_binary(&sg721::ExecuteMsg::<Empty, Empty>::Mint {
                        owner: CREATOR_ADDR.to_string(),
                        token_uri: Some("https://example.com".to_string()),
                        token_id: "1".to_string(),
                        extension: Empty {},
                    })?],
                },
                unstaking_duration: None,
                active_threshold: None,
            },
            &[],
            "cw721_voting",
            None,
        )
        .unwrap();

    let config: Config = app
        .wrap()
        .query_wasm_smart(module_addr, &QueryMsg::Config {})?;
    let sg721_addr = config.nft_address;

    // Check that the NFT contract was created
    let owner: OwnerOfResponse = app.wrap().query_wasm_smart(
        sg721_addr.clone(),
        &cw721::Cw721QueryMsg::OwnerOf {
            token_id: "1".to_string(),
            include_expired: None,
        },
    )?;
    assert_eq!(owner.owner, CREATOR_ADDR);

    // Check that collection info creator is set to the DAO (in this case CREATOR_ADDR)
    // Normally the DAO would instantiate this contract
    let creator: CollectionInfoResponse = app
        .wrap()
        .query_wasm_smart(sg721_addr, &sg721_base::msg::QueryMsg::CollectionInfo {})?;
    assert_eq!(creator.creator, CREATOR_ADDR.to_string());

    Ok(())
}

#[test]
#[should_panic(expected = "Active threshold count is greater than supply")]
fn test_instantiate_with_new_sg721_collection_abs_count_validation() {
    let mut app = StargazeApp::default();
    let module_id = app.store_code(voting_sg721_staked_contract());
    let sg721_id = app.store_code(sg721_base_contract());

    // Test edge case
    app.instantiate_contract(
        module_id,
        Addr::unchecked("contract0"),
        &InstantiateMsg {
            nft_contract: NftContract::New {
                code_id: sg721_id,
                label: "Test NFT".to_string(),
                msg: to_binary(&sg721::InstantiateMsg {
                    name: "Test NFT".to_string(),
                    symbol: "TEST".to_string(),
                    minter: "contract0".to_string(),
                    collection_info: CollectionInfo {
                        creator: "contract0".to_string(),
                        description: "Test NFT".to_string(),
                        image: "https://example.com/image.jpg".to_string(),
                        external_link: None,
                        explicit_content: None,
                        start_trading_time: None,
                        royalty_info: None,
                    },
                })
                .unwrap(),
                initial_nfts: vec![
                    to_binary(&sg721::ExecuteMsg::<Empty, Empty>::Mint {
                        owner: "contract0".to_string(),
                        token_uri: Some("https://example.com".to_string()),
                        token_id: "1".to_string(),
                        extension: Empty {},
                    })
                    .unwrap(),
                    to_binary(&sg721::ExecuteMsg::<Empty, Empty>::UpdateCollectionInfo {
                        collection_info: UpdateCollectionInfoMsg::<RoyaltyInfoResponse> {
                            description: None,
                            image: None,
                            external_link: None,
                            explicit_content: None,
                            royalty_info: None,
                        },
                    })
                    .unwrap(),
                ],
            },
            unstaking_duration: None,
            active_threshold: Some(ActiveThreshold::AbsoluteCount {
                count: Uint128::new(2),
            }),
        },
        &[],
        "cw721_voting",
        None,
    )
    .unwrap();
}
