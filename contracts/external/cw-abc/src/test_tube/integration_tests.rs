use crate::msg::ExecuteMsg;

use super::test_env::{TestEnv, TestEnvBuilder};

use cosmwasm_std::coins;
use osmosis_test_tube::OsmosisTestApp;

#[test]
fn test_happy_path() {
    let app = OsmosisTestApp::new();

    let env = TestEnvBuilder::new();
    let TestEnv { abc, accounts, .. } = env.default_setup(&app);

    // Buy tokens
    abc.execute(&ExecuteMsg::Buy {}, &coins(1000000, "uosmo"), &accounts[0])
        .unwrap();

    // TODO query curve

    // TODO burn
}
