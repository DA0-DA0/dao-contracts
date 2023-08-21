use osmosis_test_tube::OsmosisTestApp;

use crate::msg::{DenomResponse, QueryMsg};

use super::test_env::{TestEnv, TestEnvBuilder};

#[test]
fn test_create_new_denom() {
    let app = OsmosisTestApp::new();
    let env_builder = TestEnvBuilder::new();
    let TestEnv { contract, .. } = env_builder.setup(&app);

    let denom: DenomResponse = contract.query(&QueryMsg::Denom {}).unwrap();
    println!("denom: {:?}", denom);
}
