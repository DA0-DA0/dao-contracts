use crate::{
    abc::{
        ClosedConfig, CommonsPhaseConfig, HatchConfig, MinMax, OpenConfig, ReserveToken,
        SupplyToken,
    },
    msg::{CurveInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
};
use cosmwasm_std::{Coin, Decimal, Uint128};
use osmosis_test_tube::{Account, Module, OsmosisTestApp, Wasm};
use token_bindings::Metadata;

#[test]
fn test_tf() {
    // Atempt to write tests with test-tube: https://github.com/osmosis-labs/test-tube
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
    let new_admin = &accs[1];

    let wasm = Wasm::new(&app);

    let wasm_byte_code = std::fs::read("../../../artifacts/cw_abc-aarch64.wasm").unwrap();
    let code_id = wasm
        .store_code(&wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    // instantiate contract
    let contract_addr = wasm
        .instantiate(
            code_id,
            &InstantiateMsg {
                supply: SupplyToken {
                    subdenom: "subdenom".to_string(),
                    metadata: Metadata {
                        description: None,
                        denom_units: vec![],
                        base: None,
                        display: None,
                        name: None,
                        symbol: None,
                    },
                    decimals: 6,
                },
                reserve: ReserveToken {
                    denom: "ujuno".to_string(),
                    decimals: 6,
                },
                curve_type: crate::abc::CurveType::Linear {
                    slope: Uint128::new(1),
                    scale: 2,
                },
                phase_config: CommonsPhaseConfig {
                    hatch: HatchConfig {
                        initial_raise: MinMax {
                            min: Uint128::new(100),
                            max: Uint128::new(1000),
                        },
                        initial_price: Uint128::new(1),
                        initial_allocation_ratio: Decimal::percent(10),
                        exit_tax: Decimal::percent(10),
                    },
                    open: OpenConfig {
                        allocation_percentage: Decimal::percent(10),
                        exit_tax: Decimal::percent(10),
                    },
                    closed: ClosedConfig {},
                },
                hatcher_allowlist: None,
            },
            None, // contract admin used for migration, not the same as cw1_whitelist admin
            Some("cw-bounties"), // contract label
            &[],  // funds
            admin, // signer
        )
        .unwrap()
        .data
        .address;
    println!("{:?}", contract_addr);

    let curve_info = wasm
        .query::<QueryMsg, CurveInfoResponse>(&contract_addr, &QueryMsg::CurveInfo {})
        .unwrap();
    println!("{:?}", curve_info);

    // let admin_list = wasm
    //     .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
    //     .unwrap();

    // assert_eq!(admin_list.admins, init_admins);
    // assert!(admin_list.mutable);

    // // ============= NEW CODE ================

    // // update admin list and rechec the state
    // let new_admins = vec![new_admin.address()];
    // wasm.execute::<ExecuteMsg>(
    //     &contract_addr,
    //     &ExecuteMsg::UpdateAdmins {
    //         admins: new_admins.clone(),
    //     },
    //     &[],
    //     admin,
    // )
    // .unwrap();

    // let admin_list = wasm
    //     .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
    //     .unwrap();

    // assert_eq!(admin_list.admins, new_admins);
    // assert!(admin_list.mutable);
}
