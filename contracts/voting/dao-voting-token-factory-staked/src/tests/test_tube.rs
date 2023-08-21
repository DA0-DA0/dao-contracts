use cosmwasm_std::{coins, Uint128};
use cw_tokenfactory_issuer::msg::DenomUnit;
use cw_utils::Duration;
use dao_interface::state::Admin;
use osmosis_test_tube::{Account, OsmosisTestApp, Runner};

use crate::msg::{
    DenomResponse, InitialBalance, InstantiateMsg, NewTokenInfo, QueryMsg, TokenInfo,
};

use super::test_env::{TestEnv, TestEnvBuilder, JUNO};

#[test]
fn test_create_new_denom() {
    let app = OsmosisTestApp::new();
    let env_builder = TestEnvBuilder::new();
    let TestEnv {
        creator,
        contract,
        tf_issuer,
        ..
    } = env_builder.setup(&app);

    let denom: DenomResponse = contract.query(&QueryMsg::Denom {}).unwrap();
}
