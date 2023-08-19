use cosmwasm_std::Coin;
use osmosis_test_tube::{Account, Module, OsmosisTestApp, Wasm};

#[test]
fn happy_path() {
    let app = OsmosisTestApp::new();
    let accs = app
        .init_accounts(
            &[
                Coin::new(1_000_000_000_000, "uatom"),
                Coin::new(1_000_000_000_000, "uosmo"),
            ],
            2,
        )
        .unwrap();
    let admin = &accs[0];

    let wasm = Wasm::new(&app);

    let wasm_byte_code = std::fs::read("artifacts/dao_voting_token_factory_staked.wasm").unwrap();
    let code_id = wasm
        .store_code(&wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    // ============= NEW CODE ================

    // // instantiate contract with initial admin and make admin list mutable
    // let init_admins = vec![admin.address()];
    // let contract_addr = wasm
    //     .instantiate(
    //         code_id,
    //         &InstantiateMsg {
    //             admins: init_admins.clone(),
    //             mutable: true,
    //         },
    //         None, // contract admin used for migration, not the same as cw1_whitelist admin
    //         None, // contract label
    //         &[], // funds
    //         admin, // signer
    //     )
    //     .unwrap()
    //     .data
    //     .address;

    // // query contract state to check if contract instantiation works properly
    // let admin_list = wasm
    //     .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
    //     .unwrap();

    // assert_eq!(admin_list.admins, init_admins);
    // assert!(admin_list.mutable);
}
