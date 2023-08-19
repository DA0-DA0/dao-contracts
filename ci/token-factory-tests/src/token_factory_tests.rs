use cosmwasm_std::{Coin, Uint128};
use cw_utils::Duration;
use dao_interface::state::Admin;
use dao_voting_token_factory_staked::msg::{
    DenomResponse, InitialBalance, InstantiateMsg, NewTokenInfo, QueryMsg, TokenInfo,
};
use osmosis_test_tube::{Account, Module, OsmosisTestApp, Wasm};
use token_bindings::Metadata;

const DENOM: &str = "test";

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

    let contract_addr = wasm
        .instantiate(
            code_id,
            &InstantiateMsg {
                owner: Some(Admin::CoreModule {}),
                manager: Some(admin.address()),
                token_info: TokenInfo::New(NewTokenInfo {
                    subdenom: DENOM.to_string(),
                    metadata: Some(Metadata {
                        description: Some("Awesome token, get it now!".to_string()),
                        denom_units: vec![],
                        base: None,
                        display: Some(DENOM.to_string()),
                        name: Some(DENOM.to_string()),
                        symbol: Some(DENOM.to_string()),
                    }),
                    initial_balances: vec![InitialBalance {
                        amount: Uint128::new(100),
                        mint_to_address: admin.address(),
                    }],
                    initial_dao_balance: Some(Uint128::new(900)),
                }),
                unstaking_duration: Some(Duration::Height(5)),
                active_threshold: None,
            },
            None,  // contract admin used for migration, not the same as cw1_whitelist admin
            None,  // contract label
            &[],   // funds
            admin, // signer
        )
        .unwrap()
        .data
        .address;

    // Query contract state to check if contract instantiation works properly
    let tf_denom = wasm
        .query::<QueryMsg, DenomResponse>(&contract_addr, &QueryMsg::GetDenom {})
        .unwrap();

    println!("Token Factory Denom: {:?}", tf_denom);
}
