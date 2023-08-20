use cosmwasm_std::{coins, Uint128};
use cw_tokenfactory_issuer::msg::DenomUnit;
use cw_utils::Duration;
use dao_interface::state::Admin;
use osmosis_test_tube::{Account, OsmosisTestApp, Runner};

use crate::msg::{InitialBalance, InstantiateMsg, NewTokenInfo, TokenInfo};

use super::test_env::TestEnvBuilder;

const JUNO: &str = "ujuno";
const DENOM: &str = "cat";

#[test]
fn test_create_new_denom() {
    let app = OsmosisTestApp::new();
    let accounts = app.init_accounts(&coins(2_000, JUNO), 1).unwrap();
    let owner = &accounts[0];
    let t = TestEnvBuilder::new()
        .with_instantiate_msg(InstantiateMsg {
            // TODO need to do this...
            token_issuer_code_id: 0,
            owner: Some(Admin::CoreModule {}),
            manager: Some(owner.address()),
            token_info: TokenInfo::New(NewTokenInfo {
                subdenom: DENOM.to_string(),
                metadata: Some(crate::msg::NewDenomMetadata {
                    description: "Awesome token, get it meow!".to_string(),
                    additional_denom_units: Some(vec![DenomUnit {
                        denom: "ncat".to_string(),
                        exponent: 9,
                        aliases: vec![],
                    }]),
                    display: DENOM.to_string(),
                    name: DENOM.to_string(),
                    symbol: DENOM.to_string(),
                    decimals: 6,
                }),
                initial_balances: vec![InitialBalance {
                    amount: Uint128::new(100),
                    address: owner.address(),
                }],
                initial_dao_balance: Some(Uint128::new(900)),
            }),
            unstaking_duration: Some(Duration::Height(5)),
            active_threshold: None,
        })
        .build(&app);
}
