use cosmwasm_std::{Addr, Empty};
use cw4::{MemberResponse, TotalWeightResponse};
use cw721::{NftInfoResponse, OwnerOfResponse};
use cw721_base::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::error::RolesContractError;
use crate::msg::{ExecuteExt, MetadataExt, QueryExt};

const ALICE: &str = "alice";
const BOB: &str = "bob";
const DAO: &str = "dao";

// TODO add this to DAO testing after renaming
pub fn cw721_roles_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn setup() -> (App, Addr) {
    let mut app = App::default();

    let cw721_id = app.store_code(cw721_roles_contract());
    let cw721_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(DAO),
            &InstantiateMsg {
                name: "bad kids".to_string(),
                symbol: "bad kids".to_string(),
                minter: DAO.to_string(),
            },
            &[],
            "cw721_roles".to_string(),
            None,
        )
        .unwrap();

    (app, cw721_addr)
}

pub fn query_nft_owner(
    app: &App,
    nft: &Addr,
    token_id: &str,
) -> Result<cw721::OwnerOfResponse, RolesContractError> {
    let owner = app.wrap().query_wasm_smart(
        nft,
        &QueryMsg::<QueryExt>::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: None,
        },
    )?;
    Ok(owner)
}

pub fn query_member(
    app: &App,
    nft: &Addr,
    member: &str,
    at_height: Option<u64>,
) -> Result<MemberResponse, RolesContractError> {
    let member = app.wrap().query_wasm_smart(
        nft,
        &QueryMsg::<QueryExt>::Extension {
            msg: QueryExt::Member {
                addr: member.to_string(),
                at_height,
            },
        },
    )?;
    Ok(member)
}

pub fn query_total_weight(
    app: &App,
    nft: &Addr,
    at_height: Option<u64>,
) -> Result<TotalWeightResponse, RolesContractError> {
    let member = app.wrap().query_wasm_smart(
        nft,
        &QueryMsg::<QueryExt>::Extension {
            msg: QueryExt::TotalWeight { at_height },
        },
    )?;
    Ok(member)
}

pub fn query_token_info(
    app: &App,
    nft: &Addr,
    token_id: &str,
) -> Result<NftInfoResponse<MetadataExt>, RolesContractError> {
    let info = app.wrap().query_wasm_smart(
        nft,
        &QueryMsg::<QueryExt>::NftInfo {
            token_id: token_id.to_string(),
        },
    )?;
    Ok(info)
}

#[test]
fn test_minting_and_burning() {
    let (mut app, cw721_addr) = setup();

    // Mint token
    let msg = ExecuteMsg::<MetadataExt, ExecuteExt>::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt { weight: 1 },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token was created successfully
    let info: NftInfoResponse<MetadataExt> = query_token_info(&app, &cw721_addr, "1").unwrap();
    assert_eq!(info.extension.weight, 1);

    // Create another token for alice to give her even more total weight
    let msg = ExecuteMsg::<MetadataExt, ExecuteExt>::Mint {
        token_id: "2".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt { weight: 1 },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Member query returns total weight for alice
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(2));

    // Total weight is now 2
    let total: TotalWeightResponse = query_total_weight(&app, &cw721_addr, None).unwrap();
    assert_eq!(total.weight, 2);

    // Burn a role for alice
    let msg = ExecuteMsg::<MetadataExt, ExecuteExt>::Burn {
        token_id: "2".to_string(),
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token is now gone
    let res = query_token_info(&app, &cw721_addr, "2");
    assert!(res.is_err());

    // Alice's weight has been update acordingly
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(1));
}

#[test]
fn test_permissions() {
    let (mut app, cw721_addr) = setup();

    // Mint token
    let msg = ExecuteMsg::<MetadataExt, ExecuteExt>::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt { weight: 1 },
    };
    // Non-minter can't mint
    app.execute_contract(Addr::unchecked(ALICE), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // DAO can mint successfully as the minter
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Non-minter can't transfer
    let msg = ExecuteMsg::<MetadataExt, ExecuteExt>::TransferNft {
        recipient: BOB.to_string(),
        token_id: "1".to_string(),
    };
    app.execute_contract(Addr::unchecked(ALICE), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // DAO can transfer
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    let owner: OwnerOfResponse = query_nft_owner(&app, &cw721_addr, "1").unwrap();
    assert_eq!(owner.owner, BOB);
}
