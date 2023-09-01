use crate::msg::{
    CommonsPhaseConfigResponse, CurveInfoResponse, DenomResponse, ExecuteMsg, QueryMsg,
};

use super::test_env::{TestEnv, TestEnvBuilder};

use cosmwasm_std::coins;
use cw_tokenfactory_issuer::msg::QueryMsg as IssuerQueryMsg;
use osmosis_test_tube::{Account, OsmosisTestApp};

#[test]
fn test_happy_path() {
    let app = OsmosisTestApp::new();

    let builder = TestEnvBuilder::new();
    let env = builder.default_setup(&app);
    let TestEnv {
        ref abc,
        ref accounts,
        ref tf_issuer,
        ..
    } = env;

    // Buy tokens
    abc.execute(&ExecuteMsg::Buy {}, &coins(1000000, "uosmo"), &accounts[0])
        .unwrap();

    // Query denom
    let denom = tf_issuer
        .query::<DenomResponse>(&IssuerQueryMsg::Denom {})
        .unwrap()
        .denom;
    println!("Denom {:?}", denom);

    // Query balances
    let balances = env.bank().query_all_balances(
        &osmosis_test_tube::osmosis_std::types::cosmos::bank::v1beta1::QueryAllBalancesRequest {
            address: accounts[0].address(),
            pagination: None,
        },
    ).unwrap();
    println!("{:?}", balances);

    // Query curve
    let curve_info: CurveInfoResponse = abc.query(&QueryMsg::CurveInfo {}).unwrap();
    println!("Curve {:?}", curve_info);

    let phase: CommonsPhaseConfigResponse = abc.query(&QueryMsg::PhaseConfig {}).unwrap();
    println!("Phase {:?}", phase);

    // Burn
    abc.execute(&ExecuteMsg::Burn {}, &coins(900000, denom), &accounts[0])
        .unwrap();

    let curve_info: CurveInfoResponse = abc.query(&QueryMsg::CurveInfo {}).unwrap();
    println!("Curve {:?}", curve_info);
}
