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
fn test_happy_path() {
    let app = OsmosisTestApp::new();

    // TODO
}
