use cosmwasm_std::Addr;
use cw4::{MemberResponse, TotalWeightResponse};
use cw721::{NftInfoResponse, OwnerOfResponse};
use cw721_base::InstantiateMsg;
use cw_multi_test::{App, Executor};
use dao_testing::contracts::cw721_roles_contract;

use crate::error::RolesContractError;
use crate::msg::{ExecuteExt, ExecuteMsg, MetadataExt, QueryExt, QueryMsg};

const ALICE: &str = "alice";
const BOB: &str = "bob";
const DAO: &str = "dao";

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
        &QueryMsg::OwnerOf {
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
        &QueryMsg::Extension {
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
        &QueryMsg::Extension {
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
        &QueryMsg::NftInfo {
            token_id: token_id.to_string(),
        },
    )?;
    Ok(info)
}

#[test]
fn test_minting_and_burning() {
    let (mut app, cw721_addr) = setup();

    // Mint token
    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 1,
        },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token was created successfully
    let info: NftInfoResponse<MetadataExt> = query_token_info(&app, &cw721_addr, "1").unwrap();
    assert_eq!(info.extension.weight, 1);

    // Create another token for alice to give her even more total weight
    let msg = ExecuteMsg::Mint {
        token_id: "2".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 1,
        },
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
    let msg = ExecuteMsg::Burn {
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
    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: Some("member".to_string()),
            weight: 1,
        },
    };

    // Non-minter can't mint
    app.execute_contract(Addr::unchecked(ALICE), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // DAO can mint successfully as the minter
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Non-minter can't transfer
    let msg = ExecuteMsg::TransferNft {
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

#[test]
fn test_update_token_role() {
    let (mut app, cw721_addr) = setup();

    // Mint token
    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 1,
        },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::Extension {
        msg: ExecuteExt::UpdateTokenRole {
            token_id: "1".to_string(),
            role: Some("queen".to_string()),
        },
    };

    // Only admin / minter can update role
    app.execute_contract(Addr::unchecked(ALICE), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // Update token role
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token was updated successfully
    let info: NftInfoResponse<MetadataExt> = query_token_info(&app, &cw721_addr, "1").unwrap();
    assert_eq!(info.extension.role, Some("queen".to_string()));
}

#[test]
fn test_update_token_uri() {
    let (mut app, cw721_addr) = setup();

    // Mint token
    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 1,
        },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::Extension {
        msg: ExecuteExt::UpdateTokenUri {
            token_id: "1".to_string(),
            token_uri: Some("ipfs://abc...".to_string()),
        },
    };

    // Only admin / minter can update token_uri
    app.execute_contract(Addr::unchecked(ALICE), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // Update token_uri
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token was updated successfully
    let info: NftInfoResponse<MetadataExt> = query_token_info(&app, &cw721_addr, "1").unwrap();
    assert_eq!(info.token_uri, Some("ipfs://abc...".to_string()));
}

#[test]
fn test_update_token_weight() {
    let (mut app, cw721_addr) = setup();

    // Mint token
    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 1,
        },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::Extension {
        msg: ExecuteExt::UpdateTokenWeight {
            token_id: "1".to_string(),
            weight: 2,
        },
    };

    // Only admin / minter can update token weight
    app.execute_contract(Addr::unchecked(ALICE), cw721_addr.clone(), &msg, &[])
        .unwrap_err();

    // Update token weight
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token was updated successfully
    let info: NftInfoResponse<MetadataExt> = query_token_info(&app, &cw721_addr, "1").unwrap();
    assert_eq!(info.extension.weight, 2);

    // New value should be reflected in member's voting weight
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(2));

    // Update weight to a smaller value
    app.execute_contract(
        Addr::unchecked(DAO),
        cw721_addr.clone(),
        &ExecuteMsg::Extension {
            msg: ExecuteExt::UpdateTokenWeight {
                token_id: "1".to_string(),
                weight: 1,
            },
        },
        &[],
    )
    .unwrap();

    // New value should be reflected in member's voting weight
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(1));

    // Create another token for alice to give her even more total weight
    let msg = ExecuteMsg::Mint {
        token_id: "2".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 10,
        },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Alice's weight should be updated to include both tokens
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(11));

    // Update Alice's second token to 0 weight
    // Update weight to a smaller value
    app.execute_contract(
        Addr::unchecked(DAO),
        cw721_addr.clone(),
        &ExecuteMsg::Extension {
            msg: ExecuteExt::UpdateTokenWeight {
                token_id: "2".to_string(),
                weight: 0,
            },
        },
        &[],
    )
    .unwrap();

    // Alice's voting value should be 1
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(1));
}

#[test]
fn test_zero_weight_token() {
    let (mut app, cw721_addr) = setup();

    // Mint token with zero weight
    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: ALICE.to_string(),
        token_uri: Some("ipfs://xyz...".to_string()),
        extension: MetadataExt {
            role: None,
            weight: 0,
        },
    };
    app.execute_contract(Addr::unchecked(DAO), cw721_addr.clone(), &msg, &[])
        .unwrap();

    // Token was created successfully
    let info: NftInfoResponse<MetadataExt> = query_token_info(&app, &cw721_addr, "1").unwrap();
    assert_eq!(info.extension.weight, 0);

    // Member query returns total weight for alice
    let member: MemberResponse = query_member(&app, &cw721_addr, ALICE, None).unwrap();
    assert_eq!(member.weight, Some(0));
}
